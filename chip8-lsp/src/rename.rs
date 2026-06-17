use tower_lsp::lsp_types::*;

use crate::workspace::Workspace;
use chip8_asm::lexer::Token;

fn find_token_at<'a>(tokens: &'a [(Token, usize, usize)], line: u32, col: u32) -> Option<(&'a Token, usize, usize)> {
    for (tok, l, c) in tokens {
        let end = c + match tok {
            Token::Word(w) => w.len(),
            Token::Number(_) => 6,
            Token::String(s) => s.len() + 2,
            _ => 1,
        };
        if *l == line as usize && *c <= col as usize && col as usize <= end {
            return Some((tok, *l, *c));
        }
    }
    None
}

pub fn prepare_rename(ws: &Workspace, uri: &Url, pos: Position) -> Option<PrepareRenameResponse> {
    let doc = ws.get_document(uri)?;
    let tokens = doc.tokens.as_ref()?;
    let (tok, _line, _col) = find_token_at(tokens, pos.line, pos.character)?;

    match tok {
        Token::Word(w) => {
            let sym = doc.symbol_table.as_ref()?;
            if sym.resolve(w).is_none() {
                return None;
            }
            let upper = w.to_uppercase();
            let is_instr = matches!(
                upper.as_str(),
                "CLS" | "RET" | "SYS" | "JP" | "CALL" | "SE" | "SNE" | "LD" | "ADD"
                | "OR" | "AND" | "XOR" | "SUB" | "SUBN" | "SHR" | "SHL" | "RND" | "DRW"
                | "SKP" | "SKNP"
            );
            if is_instr {
                return None;
            }

            Some(PrepareRenameResponse::Range(Range {
                start: Position { line: pos.line, character: (_col as u32).saturating_sub(1) },
                end: Position { line: pos.line, character: (_col + w.len()) as u32 },
            }))
        }
        _ => None,
    }
}

pub fn perform_rename(ws: &Workspace, uri: &Url, pos: Position, new_name: &str) -> Option<WorkspaceEdit> {
    let doc = ws.get_document(uri)?;
    let tokens = doc.tokens.as_ref()?;
    let (tok, _line, _col) = find_token_at(tokens, pos.line, pos.character)?;

    match tok {
        Token::Word(w) => {
            let sym = doc.symbol_table.as_ref()?;
            if sym.resolve(w).is_none() {
                return None;
            }

            let target = w.to_uppercase();
            let mut changes: Vec<TextEdit> = Vec::new();

            for (_doc_uri, other_doc) in &ws.documents {
                let Some(other_tokens) = &other_doc.tokens else { continue };

                for (tok, line, col) in other_tokens {
                    if let Token::Word(word) = tok {
                        if word.to_uppercase() == target {
                            changes.push(TextEdit {
                                range: Range {
                                    start: Position { line: *line as u32, character: *col as u32 },
                                    end: Position { line: *line as u32, character: (*col + word.len()) as u32 },
                                },
                                new_text: new_name.to_string(),
                            });
                        }
                    }
                }
            }

            let mut map = HashMap::new();
            map.insert(uri.clone(), changes);
            Some(WorkspaceEdit {
                changes: Some(map),
                document_changes: None,
                change_annotations: None,
            })
        }
        _ => None,
    }
}

use std::collections::HashMap;
