use crate::lexer::Token;

#[derive(Debug, Clone)]
pub enum Addr {
    Num(u16),
    Label(String),
}

#[derive(Debug, Clone)]
pub enum Imm {
    Val(u16),
    Label(String),
}

#[derive(Debug, Clone)]
pub enum Inst {
    Cls,
    Ret,
    Jp(Addr),
    Call(Addr),
    SeVb(u8, Imm),
    SneVb(u8, Imm),
    SeVV(u8, u8),
    LdVb(u8, Imm),
    AddVb(u8, Imm),
    LdVV(u8, u8),
    Or(u8, u8),
    And(u8, u8),
    Xor(u8, u8),
    AddVV(u8, u8),
    Sub(u8, u8),
    Shr(u8),
    Subn(u8, u8),
    Shl(u8),
    SneVV(u8, u8),
    LdI(Addr),
    JpV0(Addr),
    Rnd(u8, Imm),
    Drw(u8, u8, Imm),
    Skp(u8),
    Sknp(u8),
    LdVdt(u8),
    LdK(u8),
    LdDt(u8),
    LdSt(u8),
    AddI(u8),
    LdF(u8),
    LdB(u8),
    LdIV(u8),
    LdVI(u8),
}

#[derive(Debug, Clone)]
pub enum Statement {
    Org(u16),
    Const(String, u16),
    Label(String),
    Inst(Inst),
    Byte(Vec<u8>),
    Word(Vec<u16>),
    Ascii(String),
    Asciz(String),
    Align(u8),
    Space(u16),
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ParseError {
    UnknownMnemonic(String, usize, usize),
    WrongArgCount { mnemonic: String, expected: usize, got: usize, line: usize, col: usize },
    ExpectedRegister(usize, usize),
    ExpectedImmediate(usize, usize),
    ExpectedIdent(usize, usize),
    ExpectedValue(usize, usize),
    InvalidRegister(String, usize, usize),
    InvalidDirective(String, usize, usize),
    UnexpectedToken(String, usize, usize),
    DuplicateLabel(String, usize),
    MissingLabelName(usize, usize),
    BadConstSyntax(usize, usize),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnknownMnemonic(m, ..) => write!(f, "unknown mnemonic '{}'", m),
            ParseError::WrongArgCount { mnemonic, expected, got, .. } => {
                write!(f, "{} expects {} operands, got {}", mnemonic, expected, got)
            }
            ParseError::ExpectedRegister(..) => write!(f, "expected register"),
            ParseError::ExpectedImmediate(..) => write!(f, "expected immediate value"),
            ParseError::ExpectedIdent(..) => write!(f, "expected identifier"),
            ParseError::ExpectedValue(..) => write!(f, "expected value"),
            ParseError::InvalidRegister(r, ..) => write!(f, "invalid register '{}'", r),
            ParseError::InvalidDirective(d, ..) => write!(f, "unknown directive '{}'", d),
            ParseError::UnexpectedToken(t, ..) => write!(f, "unexpected token '{}'", t),
            ParseError::DuplicateLabel(l, ..) => write!(f, "duplicate label '{}'", l),
            ParseError::MissingLabelName(..) => write!(f, "missing label name"),
            ParseError::BadConstSyntax(..) => write!(f, "bad .const syntax"),
        }
    }
}

pub fn parse(tokens: &[(Token, usize, usize)]) -> Result<Vec<Statement>, Vec<ParseError>> {
    let mut stmts = Vec::new();
    let mut errors = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        if matches!(tokens[i].0, Token::Eof) { break; }
        skip_newlines(tokens, &mut i);
        if i >= tokens.len() { break; }

        match parse_line(tokens, &mut i) {
            Ok(mut line_stmts) => stmts.append(&mut line_stmts),
            Err(e) => errors.push(e),
        }
    }

    if errors.is_empty() { Ok(stmts) } else { Err(errors) }
}

// -- helpers --

fn skip_newlines(tokens: &[(Token, usize, usize)], i: &mut usize) {
    while *i < tokens.len() && matches!(tokens[*i].0, Token::Newline) { *i += 1; }
}

fn peek<'a>(tokens: &'a [(Token, usize, usize)], i: usize) -> Option<&'a Token> {
    tokens.get(i).map(|(t, _, _)| t)
}

fn tok_pos(tokens: &[(Token, usize, usize)], i: usize) -> (usize, usize) {
    tokens.get(i).map(|(_, l, c)| (*l, *c)).unwrap_or((0, 0))
}

fn is_eol(tokens: &[(Token, usize, usize)], i: usize) -> bool {
    matches!(peek(tokens, i), None | Some(Token::Newline) | Some(Token::Eof))
}

// -- line parsing --

fn parse_line(tokens: &[(Token, usize, usize)], i: &mut usize) -> Result<Vec<Statement>, ParseError> {
    let mut stmts = Vec::new();

    // Check for label: Word followed by Colon
    if let Some(Token::Word(w)) = peek(tokens, *i) {
        if *i + 1 < tokens.len() && matches!(tokens[*i + 1].0, Token::Colon) {
            stmts.push(Statement::Label(w.clone()));
            *i += 2; // skip Word and Colon
            skip_newlines(tokens, i);
            if is_eol(tokens, *i) {
                return Ok(stmts); // just the label
            }
            // fall through to parse instruction/directive on same line
        }
    }

    // Directive: .word
    if matches!(peek(tokens, *i), Some(Token::Dot)) {
        *i += 1;
        stmts.push(parse_directive(tokens, i)?);
        return Ok(stmts);
    }

    // Instruction
    if !is_eol(tokens, *i) {
        if let Some(s) = parse_instruction_line(tokens, i)? {
            stmts.push(s);
        } else {
            // Not a word token at line start — report error and skip
            let (l, c) = tok_pos(tokens, *i);
            return Err(ParseError::UnexpectedToken(
                format!("{:?}", peek(tokens, *i).unwrap()),
                l, c,
            ));
        }
    }

    Ok(stmts)
}

// -- instruction parsing --

fn parse_instruction_line(
    tokens: &[(Token, usize, usize)],
    i: &mut usize,
) -> Result<Option<Statement>, ParseError> {
    let (word, ln, cl) = match peek(tokens, *i) {
        Some(Token::Word(w)) => {
            let (l, c) = tok_pos(tokens, *i);
            *i += 1;
            (w.clone(), l, c)
        }
        _ => return Ok(None),
    };

    let inst = parse_inst(&word, tokens, i, ln, cl)?;
    Ok(Some(Statement::Inst(inst)))
}

fn parse_inst(
    word: &str, tokens: &[(Token, usize, usize)], i: &mut usize, line: usize, col: usize,
) -> Result<Inst, ParseError> {
    match word.to_uppercase().as_str() {
        "CLS" => { expect_eol(tokens, i)?; Ok(Inst::Cls) }
        "RET" => { expect_eol(tokens, i)?; Ok(Inst::Ret) }
        "SYS" => { eat_rest(tokens, i); Ok(Inst::Cls) } // ignored
        "JP" => parse_jp(tokens, i, line, col),
        "CALL" => parse_unary_addr("CALL", tokens, i, line, col, Inst::Call),
        "SE" => parse_se(tokens, i, line, col),
        "SNE" => parse_sne(tokens, i, line, col),
        "LD" => parse_ld(tokens, i, line, col),
        "ADD" => parse_add(tokens, i, line, col),
        "OR" => parse_bin_reg("OR", tokens, i, line, col, Inst::Or),
        "AND" => parse_bin_reg("AND", tokens, i, line, col, Inst::And),
        "XOR" => parse_bin_reg("XOR", tokens, i, line, col, Inst::Xor),
        "SUB" => parse_bin_reg("SUB", tokens, i, line, col, Inst::Sub),
        "SUBN" => parse_bin_reg("SUBN", tokens, i, line, col, Inst::Subn),
        "SHR" => parse_unary_reg("SHR", tokens, i, line, col, Inst::Shr),
        "SHL" => parse_unary_reg("SHL", tokens, i, line, col, Inst::Shl),
        "RND" => parse_rnd(tokens, i, line, col),
        "DRW" => parse_drw(tokens, i, line, col),
        "SKP" => parse_unary_reg("SKP", tokens, i, line, col, Inst::Skp),
        "SKNP" => parse_unary_reg("SKNP", tokens, i, line, col, Inst::Sknp),
        m => Err(ParseError::UnknownMnemonic(m.to_string(), line, col)),
    }
}

fn parse_jp(
    tokens: &[(Token, usize, usize)], i: &mut usize, line: usize, col: usize,
) -> Result<Inst, ParseError> {
    let op1 = parse_operand(tokens, i)?;
    if matches!(peek(tokens, *i), Some(Token::Comma)) {
        *i += 1;
        let op2 = parse_operand(tokens, i)?;
        expect_eol(tokens, i)?;
        match op1 {
            Operand::Reg(0) => Ok(Inst::JpV0(parse_addr(op2, line, col)?)),
            Operand::Reg(r) => Err(ParseError::UnexpectedToken(
                format!("JP indexed jump requires V0, got V{:X}", r), line, col,
            )),
            _ => Err(ParseError::ExpectedRegister(line, col)),
        }
    } else {
        expect_eol(tokens, i)?;
        Ok(Inst::Jp(parse_addr(op1, line, col)?))
    }
}

fn parse_se(
    tokens: &[(Token, usize, usize)], i: &mut usize, line: usize, col: usize,
) -> Result<Inst, ParseError> {
    let op1 = parse_operand(tokens, i)?;
    expect_comma(tokens, i)?;
    let op2 = parse_operand(tokens, i)?;
    expect_eol(tokens, i)?;
    match (op1, op2) {
        (Operand::Reg(x), Operand::Imm(kk)) => Ok(Inst::SeVb(x, Imm::Val(kk))),
        (Operand::Reg(x), Operand::Ident(s)) => Ok(Inst::SeVb(x, Imm::Label(s))),
        (Operand::Reg(x), Operand::Reg(y)) => Ok(Inst::SeVV(x, y)),
        _ => Err(ParseError::WrongArgCount { mnemonic: "SE".into(), expected: 2, got: 2, line, col }),
    }
}

fn parse_sne(
    tokens: &[(Token, usize, usize)], i: &mut usize, line: usize, col: usize,
) -> Result<Inst, ParseError> {
    let op1 = parse_operand(tokens, i)?;
    expect_comma(tokens, i)?;
    let op2 = parse_operand(tokens, i)?;
    expect_eol(tokens, i)?;
    match (op1, op2) {
        (Operand::Reg(x), Operand::Imm(kk)) => Ok(Inst::SneVb(x, Imm::Val(kk))),
        (Operand::Reg(x), Operand::Ident(s)) => Ok(Inst::SneVb(x, Imm::Label(s))),
        (Operand::Reg(x), Operand::Reg(y)) => Ok(Inst::SneVV(x, y)),
        _ => Err(ParseError::WrongArgCount { mnemonic: "SNE".into(), expected: 2, got: 2, line, col }),
    }
}

fn parse_add(
    tokens: &[(Token, usize, usize)], i: &mut usize, line: usize, col: usize,
) -> Result<Inst, ParseError> {
    let op1 = parse_operand(tokens, i)?;
    expect_comma(tokens, i)?;
    let op2 = parse_operand(tokens, i)?;
    expect_eol(tokens, i)?;
    match (op1, op2) {
        (Operand::Reg(x), Operand::Imm(kk)) => Ok(Inst::AddVb(x, Imm::Val(kk))),
        (Operand::Reg(x), Operand::Ident(s)) => Ok(Inst::AddVb(x, Imm::Label(s))),
        (Operand::Reg(x), Operand::Reg(y)) => Ok(Inst::AddVV(x, y)),
        (Operand::I, Operand::Reg(x)) => Ok(Inst::AddI(x)),
        _ => Err(ParseError::WrongArgCount { mnemonic: "ADD".into(), expected: 2, got: 2, line, col }),
    }
}

fn parse_ld(
    tokens: &[(Token, usize, usize)], i: &mut usize, line: usize, col: usize,
) -> Result<Inst, ParseError> {
    let op1 = parse_operand(tokens, i)?;
    expect_comma(tokens, i)?;
    let op2 = parse_operand(tokens, i)?;
    expect_eol(tokens, i)?;
    match (op1, op2) {
        (Operand::Reg(x), Operand::Imm(kk)) => Ok(Inst::LdVb(x, Imm::Val(kk))),
        (Operand::Reg(x), Operand::Ident(s)) => Ok(Inst::LdVb(x, Imm::Label(s))),
        (Operand::Reg(x), Operand::Reg(y)) => Ok(Inst::LdVV(x, y)),
        (Operand::Reg(x), Operand::DT) => Ok(Inst::LdVdt(x)),
        (Operand::Reg(x), Operand::K) => Ok(Inst::LdK(x)),
        (Operand::DT, Operand::Reg(x)) => Ok(Inst::LdDt(x)),
        (Operand::ST, Operand::Reg(x)) => Ok(Inst::LdSt(x)),
        (Operand::F, Operand::Reg(x)) => Ok(Inst::LdF(x)),
        (Operand::B, Operand::Reg(x)) => Ok(Inst::LdB(x)),
        (Operand::I, op) => Ok(Inst::LdI(parse_addr(op, line, col)?)),
        (Operand::MemI, Operand::Reg(x)) => Ok(Inst::LdIV(x)),
        (Operand::Reg(x), Operand::MemI) => Ok(Inst::LdVI(x)),
        _ => Err(ParseError::WrongArgCount { mnemonic: "LD".into(), expected: 2, got: 2, line, col }),
    }
}

fn parse_rnd(
    tokens: &[(Token, usize, usize)], i: &mut usize, _line: usize, _col: usize,
) -> Result<Inst, ParseError> {
    let x = parse_reg(tokens, i)?;
    skip_comma(tokens, i);
    let kk = parse_imm(tokens, i, 0xFF)?;
    expect_eol(tokens, i)?;
    Ok(Inst::Rnd(x, kk))
}

fn parse_drw(
    tokens: &[(Token, usize, usize)], i: &mut usize, _line: usize, _col: usize,
) -> Result<Inst, ParseError> {
    let x = parse_reg(tokens, i)?;
    expect_comma(tokens, i)?;
    let y = parse_reg(tokens, i)?;
    expect_comma(tokens, i)?;
    let n = parse_imm(tokens, i, 0x0F)?;
    expect_eol(tokens, i)?;
    Ok(Inst::Drw(x, y, n))
}

fn parse_unary_addr(
    _name: &str, tokens: &[(Token, usize, usize)], i: &mut usize, line: usize, col: usize,
    f: fn(Addr) -> Inst,
) -> Result<Inst, ParseError> {
    let op = parse_operand(tokens, i)?;
    expect_eol(tokens, i)?;
    Ok(f(parse_addr(op, line, col)?))
}

fn parse_unary_reg(
    _name: &str, tokens: &[(Token, usize, usize)], i: &mut usize, _line: usize, _col: usize,
    f: fn(u8) -> Inst,
) -> Result<Inst, ParseError> {
    let x = parse_reg(tokens, i)?;
    expect_eol(tokens, i)?;
    Ok(f(x))
}

fn parse_bin_reg(
    _name: &str, tokens: &[(Token, usize, usize)], i: &mut usize, _line: usize, _col: usize,
    f: fn(u8, u8) -> Inst,
) -> Result<Inst, ParseError> {
    let x = parse_reg(tokens, i)?;
    expect_comma(tokens, i)?;
    let y = parse_reg(tokens, i)?;
    expect_eol(tokens, i)?;
    Ok(f(x, y))
}

// -- operand parsing --

#[derive(Debug)]
enum Operand {
    Reg(u8),
    Imm(u16),
    Ident(String),
    I,
    DT,
    ST,
    K,
    F,
    B,
    MemI,
}

fn parse_operand(tokens: &[(Token, usize, usize)], i: &mut usize) -> Result<Operand, ParseError> {
    let (line, col) = tok_pos(tokens, *i);
    match peek(tokens, *i).ok_or(ParseError::ExpectedValue(line, col))? {
        Token::Number(n) => {
            let val = *n;
            *i += 1;
            Ok(Operand::Imm(val))
        }
        Token::Word(w) => {
            let name = w.clone();
            *i += 1;
            match name.to_uppercase().as_str() {
                "I" => Ok(Operand::I),
                "DT" => Ok(Operand::DT),
                "ST" => Ok(Operand::ST),
                "K" => Ok(Operand::K),
                "F" => Ok(Operand::F),
                "B" => Ok(Operand::B),
                r if r.starts_with('V') && r.len() <= 3 => {
                    match u8::from_str_radix(&r[1..], 16) {
                        Ok(n) if n <= 15 => Ok(Operand::Reg(n)),
                        _ => Err(ParseError::InvalidRegister(name, line, col)),
                    }
                }
                _ => Ok(Operand::Ident(name)),
            }
        }
        Token::LBracket => {
            *i += 1;
            match peek(tokens, *i) {
                Some(Token::Word(w)) if w.to_uppercase() == "I" => { *i += 1; }
                _ => return Err(ParseError::ExpectedRegister(line, col)),
            }
            match peek(tokens, *i) {
                Some(Token::RBracket) => { *i += 1; }
                _ => return Err(ParseError::UnexpectedToken("expected ']'".into(), line, col)),
            }
            Ok(Operand::MemI)
        }
        t => Err(ParseError::UnexpectedToken(format!("{:?}", t), line, col)),
    }
}

fn parse_reg(tokens: &[(Token, usize, usize)], i: &mut usize) -> Result<u8, ParseError> {
    let (line, col) = tok_pos(tokens, *i);
    match parse_operand(tokens, i)? {
        Operand::Reg(r) => Ok(r),
        _ => Err(ParseError::ExpectedRegister(line, col)),
    }
}

fn parse_imm(tokens: &[(Token, usize, usize)], i: &mut usize, max_val: u16) -> Result<Imm, ParseError> {
    let (line, col) = tok_pos(tokens, *i);
    match parse_operand(tokens, i)? {
        Operand::Imm(n) if n <= max_val => Ok(Imm::Val(n)),
        Operand::Imm(n) => Err(ParseError::UnexpectedToken(
            format!("value {} exceeds max {}", n, max_val), line, col,
        )),
        Operand::Ident(s) => Ok(Imm::Label(s)),
        _ => Err(ParseError::ExpectedImmediate(line, col)),
    }
}

fn parse_addr(op: Operand, line: usize, col: usize) -> Result<Addr, ParseError> {
    match op {
        Operand::Imm(n) => Ok(Addr::Num(n)),
        Operand::Ident(s) => Ok(Addr::Label(s)),
        _ => Err(ParseError::ExpectedImmediate(line, col)),
    }
}

fn expect_comma(tokens: &[(Token, usize, usize)], i: &mut usize) -> Result<(), ParseError> {
    if matches!(peek(tokens, *i), Some(Token::Comma)) { *i += 1; }
    Ok(())
}

fn skip_comma(tokens: &[(Token, usize, usize)], i: &mut usize) {
    if matches!(peek(tokens, *i), Some(Token::Comma)) { *i += 1; }
}

fn expect_eol(tokens: &[(Token, usize, usize)], i: &mut usize) -> Result<(), ParseError> {
    if !is_eol(tokens, *i) {
        let (l, c) = tok_pos(tokens, *i);
        return Err(unexpected(peek(tokens, *i).unwrap(), l, c));
    }
    Ok(())
}

fn eat_rest(tokens: &[(Token, usize, usize)], i: &mut usize) {
    while *i < tokens.len() && !matches!(tokens[*i].0, Token::Newline | Token::Eof) {
        *i += 1;
    }
}

fn unexpected(t: &Token, line: usize, col: usize) -> ParseError {
    ParseError::UnexpectedToken(format!("{:?}", t), line, col)
}

// -- directive parsing --

fn parse_directive(tokens: &[(Token, usize, usize)], i: &mut usize) -> Result<Statement, ParseError> {
    let (name, line, col) = match peek(tokens, *i) {
        Some(Token::Word(w)) => {
            let (l, c) = tok_pos(tokens, *i);
            *i += 1;
            (w.clone(), l, c)
        }
        Some(t) => return Err(ParseError::InvalidDirective(format!("{:?}", t), tok_pos(tokens, *i).0, 0)),
        None => return Err(ParseError::MissingLabelName(0, 0)),
    };

    match name.to_lowercase().as_str() {
        "org"   => { let v = parse_single_imm(tokens, i, 0x0FFF)?; Ok(Statement::Org(v)) }
        "byte"  => { let v = parse_imm_list(tokens, i, 0xFF)?.into_iter().map(|x| x as u8).collect::<Vec<_>>(); Ok(Statement::Byte(v)) }
        "word"  => { let v = parse_imm_list(tokens, i, 0xFFFF)?; Ok(Statement::Word(v)) }
        "ascii" => { let s = parse_string(tokens, i)?; Ok(Statement::Ascii(s)) }
        "asciz" => { let s = parse_string(tokens, i)?; Ok(Statement::Asciz(s)) }
        "align" => { let n = parse_single_imm(tokens, i, 0xFF)? as u8; Ok(Statement::Align(n)) }
        "space" => { let n = parse_single_imm(tokens, i, 0xFFFF)?; Ok(Statement::Space(n)) }
        "const" => parse_const(tokens, i, line, col),
        d => Err(ParseError::InvalidDirective(d.to_string(), line, col)),
    }
}

fn parse_const(tokens: &[(Token, usize, usize)], i: &mut usize, line: usize, col: usize) -> Result<Statement, ParseError> {
    let cname = match peek(tokens, *i) {
        Some(Token::Word(w)) => { let name = w.clone(); *i += 1; name }
        _ => return Err(ParseError::ExpectedIdent(line, col)),
    };
    if matches!(peek(tokens, *i), Some(Token::Equals)) { *i += 1; }
    let val = parse_single_imm(tokens, i, 0xFFFF)?;
    Ok(Statement::Const(cname, val))
}

fn parse_single_imm(tokens: &[(Token, usize, usize)], i: &mut usize, max_val: u16) -> Result<u16, ParseError> {
    let (line, col) = tok_pos(tokens, *i);
    match peek(tokens, *i) {
        Some(Token::Number(n)) => {
            let val = *n;
            if val <= max_val {
                *i += 1;
                Ok(val)
            } else {
                Err(ParseError::UnexpectedToken(format!("value {} exceeds max {}", val, max_val), line, col))
            }
        }
        _ => Err(ParseError::ExpectedValue(line, col)),
    }
}

fn parse_imm_list(tokens: &[(Token, usize, usize)], i: &mut usize, max_val: u16) -> Result<Vec<u16>, ParseError> {
    skip_newlines(tokens, i);
    let mut vals = Vec::new();
    loop {
        match peek(tokens, *i) {
            Some(Token::Number(n)) => {
                if *n > max_val {
                    let (l, c) = tok_pos(tokens, *i);
                    return Err(ParseError::UnexpectedToken(format!("value {} exceeds max {}", n, max_val), l, c));
                }
                vals.push(*n);
                *i += 1;
            }
            _ => break,
        }
        skip_newlines(tokens, i);
        if !matches!(peek(tokens, *i), Some(Token::Comma)) { break; }
        *i += 1;
    }
    if vals.is_empty() {
        let (l, c) = tok_pos(tokens, *i);
        Err(ParseError::ExpectedImmediate(l, c))
    } else {
        Ok(vals)
    }
}

fn parse_string(tokens: &[(Token, usize, usize)], i: &mut usize) -> Result<String, ParseError> {
    let (line, col) = tok_pos(tokens, *i);
    match peek(tokens, *i) {
        Some(Token::String(s)) => { let v = s.clone(); *i += 1; Ok(v) }
        Some(t) => Err(ParseError::UnexpectedToken(format!("{:?}", t), line, col)),
        None => Err(ParseError::ExpectedValue(line, col)),
    }
}
