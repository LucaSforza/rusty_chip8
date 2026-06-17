use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};

pub fn extract_parse_pos(e: &chip8_asm::parser::ParseError) -> (usize, usize) {
    use chip8_asm::parser::ParseError::*;
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

pub fn translate(errors: &Option<Vec<chip8_asm::AssemblyError>>, _uri: &Url) -> Vec<Diagnostic> {
    let Some(errs) = errors else { return Vec::new() };
    errs.iter().map(|e| {
        let line = e.line as u32;
        let col = e.col as u32;
        Diagnostic {
            range: Range {
                start: Position { line, character: col },
                end: Position { line, character: col + 1 },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            message: e.message.clone(),
            source: Some("chip8-asm".into()),
            ..Default::default()
        }
    }).collect()
}
