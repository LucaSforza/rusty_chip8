use tower_lsp::lsp_types::*;

use crate::workspace::Workspace;

#[allow(deprecated)]
pub fn document_symbols(ws: &Workspace, uri: &Url) -> Option<DocumentSymbolResponse> {
    let doc = ws.get_document(uri)?;
    let sym = doc.symbol_table.as_ref()?;

    let mut symbols = Vec::new();

    for (name, addr) in sym.labels() {
        symbols.push(DocumentSymbol {
            name: name.clone(),
            kind: SymbolKind::FUNCTION,
            range: simple_range(0, 0, 0, 0),
            selection_range: simple_range(0, 0, 0, 0),
            detail: Some(format!("0x{:03X}", addr)),
            children: None,
            tags: None,
            deprecated: None,
        });
    }

    for (name, val) in sym.constants() {
        if !name.contains('.') {
            symbols.push(DocumentSymbol {
                name: name.clone(),
                kind: SymbolKind::PROPERTY,
                range: simple_range(0, 0, 0, 0),
                selection_range: simple_range(0, 0, 0, 0),
                detail: Some(format!("= {}", val)),
                children: None,
                tags: None,
                deprecated: None,
            });
        }
    }

    if symbols.is_empty() { None } else { Some(DocumentSymbolResponse::Nested(symbols)) }
}

#[allow(deprecated)]
pub fn workspace_symbols(ws: &Workspace, query: &str) -> Option<Vec<SymbolInformation>> {
    let mut results = Vec::new();
    let q = query.to_uppercase();

    for (uri, doc) in &ws.documents {
        let Some(analysis) = &doc.analysis else { continue };
        let sym = &analysis.symbol_table;

        for (name, _addr) in sym.labels() {
            if q.is_empty() || name.to_uppercase().contains(&q) {
                results.push(SymbolInformation {
                    name: name.clone(),
                    kind: SymbolKind::FUNCTION,
                    location: Location {
                        uri: uri.clone(),
                        range: simple_range(0, 0, 0, 0),
                    },
                    container_name: None,
                    tags: None,
                    deprecated: None,
                });
            }
        }

        for (name, _val) in sym.constants() {
            if !name.contains('.') && (q.is_empty() || name.to_uppercase().contains(&q)) {
                results.push(SymbolInformation {
                    name: name.clone(),
                    kind: SymbolKind::PROPERTY,
                    location: Location {
                        uri: uri.clone(),
                        range: simple_range(0, 0, 0, 0),
                    },
                    container_name: None,
                    tags: None,
                    deprecated: None,
                });
            }
        }
    }

    if results.is_empty() { None } else { Some(results) }
}

fn simple_range(line: u32, col: u32, end_line: u32, end_col: u32) -> Range {
    Range {
        start: Position { line, character: col },
        end: Position { line: end_line, character: end_col },
    }
}
