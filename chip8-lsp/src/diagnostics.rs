use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};

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
