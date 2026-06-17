use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::encoder::Instr;
use crate::lexer::Token;
use crate::parser::{Addr, Imm, Inst, Statement};
use crate::sourcemap::SourceMap;
use crate::symbol::SymbolTable;

mod sourcemap;
mod include;
mod macroexpand;
mod preprocess;
pub mod lexer;
pub mod parser;
pub mod encoder;
pub mod symbol;

pub use crate::include::{FileProvider, FsFileProvider, OverlayFileProvider};
pub use crate::preprocess::{PreprocessError, PreprocessResult};

#[derive(Debug)]
pub struct AssemblyError {
    pub message: String,
    pub file: Option<String>,
    pub line: usize,
    pub col: usize,
}

impl std::fmt::Display for AssemblyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.file {
            Some(file) => {
                write!(
                    f,
                    "{}:{}:{}: {}",
                    file,
                    self.line + 1,
                    self.col + 1,
                    self.message
                )
            }
            None => write!(f, "{}:{}: {}", self.line + 1, self.col + 1, self.message),
        }
    }
}

impl AssemblyError {
    fn from_string(msg: String) -> Self {
        AssemblyError {
            message: msg,
            file: None,
            line: 0,
            col: 0,
        }
    }
}

#[derive(Default)]
pub struct AssemblyOptions {
    pub base_dir: PathBuf,
    pub files: HashMap<PathBuf, String>,
}

#[derive(Debug, Clone)]
pub struct AssembleResult {
    pub bytes: Vec<u8>,
    pub listing: Vec<String>,
}

pub fn assemble(source: &str) -> Result<AssembleResult, Vec<AssemblyError>> {
    assemble_with(source, &AssemblyOptions::default())
}

pub fn assemble_with(
    source: &str,
    opts: &AssemblyOptions,
) -> Result<AssembleResult, Vec<AssemblyError>> {
    // 1. Preprocessing
    let pp = if opts.files.is_empty() {
        let fs = FsFileProvider;
        preprocess::preprocess(source, &opts.base_dir, &fs)
            .map_err(|errs| errs.into_iter().map(pp_error_to_assembly).collect::<Vec<_>>())?
    } else {
        let fs = FsFileProvider;
        let provider = OverlayFileProvider {
            overlay: &opts.files,
            base: &fs,
        };
        preprocess::preprocess(source, &opts.base_dir, &provider)
            .map_err(|errs| errs.into_iter().map(pp_error_to_assembly).collect::<Vec<_>>())?
    };

    // 2. Lex
    let tokens = lexer::tokenize(&pp.source);
    if let Some(errs) = check_lex_errors(&tokens) {
        return Err(errs
            .into_iter()
            .map(|(msg, l, c)| translate_error(msg, l, c, &pp.source_map))
            .collect());
    }

    // 3. Parse
    let statements = match parser::parse(&tokens) {
        Ok(s) => s,
        Err(errs) => {
            return Err(
                errs.into_iter()
                    .map(|e| {
                        let (l, c) = extract_err_pos(&e);
                        translate_error(e.to_string(), l, c, &pp.source_map)
                    })
                    .collect(),
            )
        }
    };

    // 4. Compute layout (was pass1)
    let (sym, addresses) = match compute_layout(&statements) {
        Ok(r) => r,
        Err(e) => return Err(vec![AssemblyError::from_string(e)]),
    };

    // 5. Generate code (was pass2)
    let (output, listing) = match generate_code(&statements, &addresses, &sym) {
        Ok(r) => r,
        Err(e) => return Err(vec![AssemblyError::from_string(e)]),
    };

    Ok(AssembleResult {
        bytes: output,
        listing,
    })
}

pub fn assemble_file(path: &Path) -> Result<AssembleResult, Vec<AssemblyError>> {
    let source = std::fs::read_to_string(path).map_err(|e| {
        vec![AssemblyError {
            message: format!("{}: {}", path.display(), e),
            file: None,
            line: 0,
            col: 0,
        }]
    })?;
    let base_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    assemble_with(
        &source,
        &AssemblyOptions {
            base_dir,
            ..Default::default()
        },
    )
}

fn pp_error_to_assembly(e: PreprocessError) -> AssemblyError {
    AssemblyError::from_string(e.to_string())
}

fn translate_error(msg: String, line: usize, col: usize, source_map: &SourceMap) -> AssemblyError {
    let (file, file_line) = source_map.resolve(line);
    AssemblyError {
        message: msg,
        file: Some(file.to_string()),
        line: file_line,
        col,
    }
}

fn check_lex_errors(
    tokens: &[(Token, usize, usize)],
) -> Option<Vec<(String, usize, usize)>> {
    let errs: Vec<_> = tokens
        .iter()
        .filter_map(|(t, l, c)| {
            if let Token::Error(s) = t {
                Some((s.clone(), *l, *c))
            } else {
                None
            }
        })
        .collect();
    if errs.is_empty() {
        None
    } else {
        Some(errs)
    }
}

fn extract_err_pos(e: &parser::ParseError) -> (usize, usize) {
    use parser::ParseError::*;
    match e {
        UnknownMnemonic(_, l, c) => (*l, *c),
        WrongArgCount { line, col, .. } => (*line, *col),
        ExpectedRegister(l, c) => (*l, *c),
        ExpectedImmediate(l, c) => (*l, *c),
        ExpectedIdent(l, c) => (*l, *c),
        ExpectedValue(l, c) => (*l, *c),
        InvalidRegister(_, l, c) => (*l, *c),
        InvalidDirective(_, l, c) => (*l, *c),
        UnexpectedToken(_, l, c) => (*l, *c),
        DuplicateLabel(_, l) => (*l, 0),
        MissingLabelName(l, c) => (*l, *c),
        BadConstSyntax(l, c) => (*l, *c),
    }
}

// ── compute_layout (was pass1) ──────────────────────────────────────────

fn compute_layout(
    statements: &[Statement],
) -> Result<(SymbolTable, Vec<u16>), String> {
    let mut sym = SymbolTable::new();
    let mut addr: u16 = 0x200;
    let mut addresses = Vec::new();

    for stmt in statements {
        addresses.push(addr);
        match stmt {
            Statement::Org(a) => {
                if *a > 0xFFF {
                    return Err(format!("org address 0x{:03X} exceeds 4KB limit", a));
                }
                addr = *a;
            }
            Statement::Label(name) => {
                if sym.has_label(name) {
                    return Err(format!("duplicate label '{}' at 0x{:03X}", name, addr));
                }
                sym.define_label(name, addr, 0)
                    .map_err(|_| format!("duplicate label '{}'", name))?;
            }
            Statement::Const(name, val) => {
                sym.define_const(name, *val, 0)
                    .map_err(|_| format!("duplicate constant '{}'", name))?;
            }
            Statement::Struct { name, fields } => {
                let mut offset: u16 = 0;
                for field in fields {
                    let field_name = format!("{}.{}", name, field.name);
                    sym.define_const(&field_name, offset, 0)
                        .map_err(|_| format!("duplicate constant '{}'", field_name))?;
                    offset += field.kind.size();
                }
                sym.define_const(&format!("{}.SIZE", name), offset, 0)
                    .map_err(|_| format!("duplicate constant '{}'.SIZE", name))?;
            }
            Statement::Inst(_) => {
                addr = addr.wrapping_add(2);
            }
            Statement::Byte(v) => {
                addr = addr.wrapping_add(v.len() as u16);
            }
            Statement::Word(v) => {
                addr = addr.wrapping_add((v.len() * 2) as u16);
            }
            Statement::Ascii(s) => {
                addr = addr.wrapping_add(s.len() as u16);
            }
            Statement::Asciz(s) => {
                addr = addr.wrapping_add(s.len() as u16 + 1);
            }
            Statement::Align(n) => {
                if *n > 1 {
                    let mask = (*n as u16) - 1;
                    if addr & mask != 0 {
                        addr = (addr + mask) & !mask;
                    }
                }
            }
            Statement::Space(n) => {
                addr = addr.wrapping_add(*n);
            }
        }
    }

    if addr > 0x1000 {
        return Err(format!("program extends past 4KB (0x{:04X})", addr));
    }

    Ok((sym, addresses))
}

// ── generate_code (was pass2) ───────────────────────────────────────────

fn generate_code(
    statements: &[Statement],
    addresses: &[u16],
    sym: &SymbolTable,
) -> Result<(Vec<u8>, Vec<String>), String> {
    let mut output = Vec::new();
    let mut listing = Vec::new();

    for (i, stmt) in statements.iter().enumerate() {
        let addr = addresses[i];
        match stmt {
            Statement::Org(_) | Statement::Label(_) | Statement::Const(..) | Statement::Struct { .. } => {
                // no bytes emitted
            }
            Statement::Inst(inst) => {
                let instr = resolve_inst(inst, sym)
                    .map_err(|name| format!("undefined symbol '{}'", name))?;
                let bytes = instr.encode();
                let line = format!(
                    "  {:04X}  {:02X}{:02X}    {}",
                    addr,
                    bytes[0],
                    bytes[1],
                    instr.mnemonic()
                );
                listing.push(line);
                output.extend_from_slice(&bytes);
            }
            Statement::Byte(v) => {
                let mut resolved = Vec::new();
                for imm in v {
                    let val = resolve_imm(imm, sym, 0xFF)? as u8;
                    resolved.push(val);
                }
                let hex: Vec<String> = resolved.iter().map(|b| format!("{:02X}", b)).collect();
                let line = format!("  {:04X}  {:23}  .byte", addr, hex.join(" "));
                listing.push(line);
                output.extend_from_slice(&resolved);
            }
            Statement::Word(v) => {
                let mut resolved = Vec::new();
                for imm in v {
                    let val = resolve_imm(imm, sym, 0xFFFF)?;
                    resolved.push(val);
                }
                let hex: Vec<String> = resolved.iter().map(|w| format!("{:04X}", w)).collect();
                let line = format!("  {:04X}  {:23}  .word", addr, hex.join(" "));
                listing.push(line);
                for w in &resolved {
                    output.extend_from_slice(&w.to_be_bytes());
                }
            }
            Statement::Ascii(s) => {
                let hex: Vec<String> =
                    s.bytes().map(|b| format!("{:02X}", b)).collect();
                let line = format!(
                    "  {:04X}  {:23}  .ascii \"{}\"",
                    addr,
                    hex.join(" "),
                    s.escape_default()
                );
                listing.push(line);
                output.extend_from_slice(s.as_bytes());
            }
            Statement::Asciz(s) => {
                let hex: Vec<String> = s
                    .bytes()
                    .chain(std::iter::once(0))
                    .map(|b| format!("{:02X}", b))
                    .collect();
                let line = format!(
                    "  {:04X}  {:23}  .asciz \"{}\"",
                    addr,
                    hex.join(" "),
                    s.escape_default()
                );
                listing.push(line);
                output.extend_from_slice(s.as_bytes());
                output.push(0);
            }
            Statement::Align(n) => {
                let mask = (*n as u16) - 1;
                if *n > 1 && addr & mask != 0 {
                    let pad_count = (mask + 1) - (addr & mask);
                    let mut addr2 = addr;
                    for _ in 0..pad_count {
                        let line = format!(
                            "  {:04X}  00                     .align {}",
                            addr2, n
                        );
                        listing.push(line);
                        output.push(0);
                        addr2 += 1;
                    }
                }
            }
            Statement::Space(n) => {
                let line = format!("  {:04X}                        .space {}", addr, n);
                listing.push(line);
                output.extend(std::iter::repeat_n(0, *n as usize));
            }
        }
    }

    Ok((output, listing))
}

fn resolve_imm(imm: &Imm, sym: &SymbolTable, max_val: u16) -> Result<u16, String> {
    let val = match imm {
        Imm::Val(n) => *n,
        Imm::Label(name) => sym.resolve(name).ok_or_else(|| name.clone())?,
    };
    if val > max_val {
        return Err(format!("value {} exceeds max {}", val, max_val));
    }
    Ok(val)
}

fn resolve_inst(inst: &Inst, sym: &SymbolTable) -> Result<Instr, String> {
    let r = |addr: &Addr| -> Result<u16, String> {
        match addr {
            Addr::Num(n) => Ok(*n),
            Addr::Label(name) => sym.resolve(name).ok_or_else(|| name.clone()),
        }
    };

    Ok(match inst {
        Inst::Cls => Instr::Cls,
        Inst::Ret => Instr::Ret,
        Inst::Jp(a) => Instr::Jp(r(a)?),
        Inst::Call(a) => Instr::Call(r(a)?),
        Inst::SeVb(x, imm) => Instr::SeVb(*x, resolve_imm(imm, sym, 0xFF)? as u8),
        Inst::SneVb(x, imm) => Instr::SneVb(*x, resolve_imm(imm, sym, 0xFF)? as u8),
        Inst::SeVV(x, y) => Instr::SeVV(*x, *y),
        Inst::LdVb(x, imm) => Instr::LdVb(*x, resolve_imm(imm, sym, 0xFF)? as u8),
        Inst::AddVb(x, imm) => Instr::AddVb(*x, resolve_imm(imm, sym, 0xFF)? as u8),
        Inst::LdVV(x, y) => Instr::LdVV(*x, *y),
        Inst::Or(x, y) => Instr::Or(*x, *y),
        Inst::And(x, y) => Instr::And(*x, *y),
        Inst::Xor(x, y) => Instr::Xor(*x, *y),
        Inst::AddVV(x, y) => Instr::AddVV(*x, *y),
        Inst::Sub(x, y) => Instr::Sub(*x, *y),
        Inst::Shr(x) => Instr::Shr(*x),
        Inst::Subn(x, y) => Instr::Subn(*x, *y),
        Inst::Shl(x) => Instr::Shl(*x),
        Inst::SneVV(x, y) => Instr::SneVV(*x, *y),
        Inst::LdI(a) => Instr::LdI(r(a)?),
        Inst::JpV0(a) => Instr::JpV0(r(a)?),
        Inst::Rnd(x, imm) => Instr::Rnd(*x, resolve_imm(imm, sym, 0xFF)? as u8),
        Inst::Drw(x, y, imm) => Instr::Drw(*x, *y, resolve_imm(imm, sym, 0xF)? as u8),
        Inst::Skp(x) => Instr::Skp(*x),
        Inst::Sknp(x) => Instr::Sknp(*x),
        Inst::LdVdt(x) => Instr::LdVdt(*x),
        Inst::LdK(x) => Instr::LdK(*x),
        Inst::LdDt(x) => Instr::LdDt(*x),
        Inst::LdSt(x) => Instr::LdSt(*x),
        Inst::AddI(x) => Instr::AddI(*x),
        Inst::LdF(x) => Instr::LdF(*x),
        Inst::LdB(x) => Instr::LdB(*x),
        Inst::LdIV(x) => Instr::LdIV(*x),
        Inst::LdVI(x) => Instr::LdVI(*x),
    })
}
