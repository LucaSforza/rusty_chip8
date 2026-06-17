use tower_lsp::lsp_types::*;

use crate::workspace::Workspace;
use chip8_asm::lexer::Token;

fn find_token_at<'a>(tokens: &'a [(Token, usize, usize)], line: u32, col: u32) -> Option<(&'a Token, usize, usize)> {
    for (tok, l, c) in tokens {
        let end = c + match tok {
            Token::Word(w) => w.len(),
            Token::Number(n) => n.to_string().len() + 2,
            Token::String(s) => s.len() + 2,
            _ => 1,
        };
        if *l == line as usize && *c <= col as usize && col as usize <= end {
            return Some((tok, *l, *c));
        }
    }
    None
}

pub fn goto_definition(ws: &Workspace, uri: &Url, pos: Position) -> Option<GotoDefinitionResponse> {
    let doc = ws.get_document(uri)?;
    let tokens = doc.tokens.as_ref()?;
    let analysis = doc.analysis.as_ref()?;

    let (tok, _line, _col) = find_token_at(tokens, pos.line, pos.character)?;

    match tok {
        Token::Word(w) => {
            let sym = doc.symbol_table.as_ref()?;

            // Skip if it's an instruction, register, or directive keyword
            let upper = w.to_uppercase();
            let is_instr = matches!(
                upper.as_str(),
                "CLS" | "RET" | "SYS" | "JP" | "CALL" | "SE" | "SNE" | "LD" | "ADD"
                | "OR" | "AND" | "XOR" | "SUB" | "SUBN" | "SHR" | "SHL" | "RND" | "DRW"
                | "SKP" | "SKNP"
            );
            let is_reg = matches!(
                upper.as_str(),
                "I" | "DT" | "ST" | "K" | "F" | "B"
            ) || (upper.starts_with('V') && upper[1..].parse::<u8>().ok().map_or(false, |n| n <= 0xF));
            let is_directive = matches!(
                upper.as_str(),
                "ORG" | "BYTE" | "WORD" | "ASCII" | "ASCIZ" | "ALIGN" | "SPACE" | "CONST" | "STRUCT"
            );

            if is_instr || is_reg || is_directive {
                return None;
            }

            // Try to resolve as symbol
            if let Some(_addr) = sym.resolve(w) {
                // Find where this symbol is defined in the source
                let source = &analysis.source;
                for (line_idx, line) in source.lines().enumerate() {
                    let trimmed = line.trim();
                    let is_label_def = trimmed == format!("{}:", w)
                        || trimmed.starts_with(&format!("{}:", w));
                    let is_const_def = trimmed.contains(".const")
                        && trimmed.contains(w)
                        && trimmed.contains('=');
                    if is_label_def || is_const_def {
                        let loc = Location {
                            uri: uri.clone(),
                            range: Range {
                                start: Position { line: line_idx as u32, character: 0 },
                                end: Position { line: line_idx as u32, character: line.len() as u32 },
                            },
                        };
                        return Some(GotoDefinitionResponse::Scalar(loc));
                    }
                }
            }
        }
        _ => {}
    }

    None
}
