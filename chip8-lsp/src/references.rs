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

pub fn find_references(ws: &Workspace, uri: &Url, pos: Position) -> Option<Vec<Location>> {
    let doc = ws.get_document(uri)?;
    let tokens = doc.tokens.as_ref()?;

    let (tok, _line, _col) = find_token_at(tokens, pos.line, pos.character)?;

    let w = match tok {
        Token::Word(w) => w.clone(),
        _ => return None,
    };

    let target = w.to_uppercase();

    let mut locations = Vec::new();

    for (doc_uri, other_doc) in &ws.documents {
        let Some(other_tokens) = &other_doc.tokens else { continue };

        for (tok, line, col) in other_tokens {
            if let Token::Word(word) = tok {
                if word.to_uppercase() == target {
                    locations.push(Location {
                        uri: doc_uri.clone(),
                        range: Range {
                            start: Position { line: *line as u32, character: *col as u32 },
                            end: Position { line: *line as u32, character: (*col + word.len()) as u32 },
                        },
                    });
                }
            }
        }
    }

    if locations.is_empty() { None } else { Some(locations) }
}
