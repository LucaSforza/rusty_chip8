use std::path::PathBuf;
use clap::Parser;
use crate::encoder::Instr;
use crate::lexer::Token;
use crate::parser::{Addr, Inst, Statement};
use crate::symbol::SymbolTable;

mod lexer;
mod parser;
mod encoder;
mod symbol;

#[derive(Parser)]
#[command(name = "chip8-asm", about = "CHIP-8 assembler")]
struct Cli {
    input: PathBuf,
    #[arg(short = 'o', long, default_value = "a.out.ch8")]
    output: PathBuf,
    #[arg(short = 'l', long)]
    listing: Option<PathBuf>,
}

fn main() {
    let args = Cli::parse();

    let source = match std::fs::read_to_string(&args.input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}: {}", args.input.display(), e);
            std::process::exit(1);
        }
    };

    let tokens = lexer::tokenize(&source);

    if let Some(errs) = check_lex_errors(&tokens) {
        for e in &errs {
            let (tok, line, col) = e;
            eprintln!("  {}:{}: lex error: {}", line + 1, col + 1, tok);
        }
        std::process::exit(1);
    }

    let statements = match parser::parse(&tokens) {
        Ok(s) => s,
        Err(errs) => {
            for e in &errs {
                let (line, col) = extract_err_pos(e);
                eprintln!("  {}:{}: {}", line + 1, col + 1, e);
            }
            std::process::exit(1);
        }
    };

    let (sym, addresses) = match pass1(&statements) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    let (output, listing) = match pass2(&statements, &addresses, &sym) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: undefined symbol '{}'", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = std::fs::write(&args.output, &output) {
        eprintln!("error: writing {}: {}", args.output.display(), e);
        std::process::exit(1);
    }
    println!("wrote {} bytes to {}", output.len(), args.output.display());

    if let Some(list_path) = args.listing {
        if let Err(e) = std::fs::write(&list_path, listing.join("\n")) {
            eprintln!("error: writing {}: {}", list_path.display(), e);
        }
    }
}

fn check_lex_errors(tokens: &[(Token, usize, usize)]) -> Option<Vec<(String, usize, usize)>> {
    let errs: Vec<_> = tokens.iter().filter_map(|(t, l, c)| {
        if let Token::Error(s) = t {
            Some((s.clone(), *l, *c))
        } else {
            None
        }
    }).collect();
    if errs.is_empty() { None } else { Some(errs) }
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

// Pass 1: build symbol table and compute statement addresses
fn pass1(statements: &[Statement]) -> Result<(SymbolTable, Vec<u16>), String> {
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
                sym.define_label(name, addr, 0).map_err(|_| format!("duplicate label '{}'", name))?;
            }
            Statement::Const(name, val) => {
                sym.define_const(name, *val, 0).map_err(|_| format!("duplicate constant '{}'", name))?;
            }
            Statement::Inst(_) => { addr = addr.wrapping_add(2); }
            Statement::Byte(v) => { addr = addr.wrapping_add(v.len() as u16); }
            Statement::Word(v) => { addr = addr.wrapping_add((v.len() * 2) as u16); }
            Statement::Ascii(s) => { addr = addr.wrapping_add(s.len() as u16); }
            Statement::Asciz(s) => { addr = addr.wrapping_add(s.len() as u16 + 1); }
            Statement::Align(n) => {
                if *n > 1 {
                    let mask = (*n as u16) - 1;
                    if addr & mask != 0 {
                        addr = (addr + mask) & !mask;
                    }
                }
            }
            Statement::Space(n) => { addr = addr.wrapping_add(*n); }
        }
    }

    // Verify final address is within range
    if addr > 0x1000 {
        return Err(format!("program extends past 4KB (0x{:04X})", addr));
    }

    Ok((sym, addresses))
}

// Pass 2: resolve labels and emit bytes
fn pass2(
    statements: &[Statement],
    addresses: &[u16],
    sym: &SymbolTable,
) -> Result<(Vec<u8>, Vec<String>), String> {
    let mut output = Vec::new();
    let mut listing = Vec::new();

    for (i, stmt) in statements.iter().enumerate() {
        let addr = addresses[i];
        match stmt {
            Statement::Org(_) | Statement::Label(_) | Statement::Const(..) => {
                // no bytes emitted for these
            }
            Statement::Inst(inst) => {
                let instr = resolve_inst(inst, sym)
                    .map_err(|name| format!("undefined symbol '{}'", name))?;
                let bytes = instr.encode();
                let line = format!("  {:04X}  {:02X}{:02X}    {}", addr, bytes[0], bytes[1], instr.mnemonic());
                listing.push(line);
                output.extend_from_slice(&bytes);
            }
            Statement::Byte(v) => {
                let hex: Vec<String> = v.iter().map(|b| format!("{:02X}", b)).collect();
                let line = format!("  {:04X}  {:23}  .byte", addr, hex.join(" "));
                listing.push(line);
                output.extend_from_slice(v);
            }
            Statement::Word(v) => {
                let hex: Vec<String> = v.iter().map(|w| format!("{:04X}", w)).collect();
                let line = format!("  {:04X}  {:23}  .word", addr, hex.join(" "));
                listing.push(line);
                for w in v {
                    output.extend_from_slice(&w.to_be_bytes());
                }
            }
            Statement::Ascii(s) => {
                let hex: Vec<String> = s.bytes().map(|b| format!("{:02X}", b)).collect();
                let line = format!("  {:04X}  {:23}  .ascii \"{}\"", addr, hex.join(" "), s.escape_default());
                listing.push(line);
                output.extend_from_slice(s.as_bytes());
            }
            Statement::Asciz(s) => {
                let hex: Vec<String> = s.bytes().chain(std::iter::once(0)).map(|b| format!("{:02X}", b)).collect();
                let line = format!("  {:04X}  {:23}  .asciz \"{}\"", addr, hex.join(" "), s.escape_default());
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
                        let line = format!("  {:04X}  00                     .align {}", addr2, n);
                        listing.push(line);
                        output.push(0);
                        addr2 += 1;
                    }
                }
            }
            Statement::Space(n) => {
                let line = format!("  {:04X}                        .space {}", addr, n);
                listing.push(line);
                for _ in 0..*n { output.push(0); }
            }
        }
    }

    Ok((output, listing))
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
        Inst::SeVb(x, kk) => Instr::SeVb(*x, *kk),
        Inst::SneVb(x, kk) => Instr::SneVb(*x, *kk),
        Inst::SeVV(x, y) => Instr::SeVV(*x, *y),
        Inst::LdVb(x, kk) => Instr::LdVb(*x, *kk),
        Inst::AddVb(x, kk) => Instr::AddVb(*x, *kk),
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
        Inst::Rnd(x, kk) => Instr::Rnd(*x, *kk),
        Inst::Drw(x, y, n) => Instr::Drw(*x, *y, *n),
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
