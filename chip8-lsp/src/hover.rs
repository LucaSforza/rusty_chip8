use tower_lsp::lsp_types::*;

use crate::workspace::Workspace;
use chip8_asm::lexer::Token;

fn find_token<'a>(tokens: &'a [(Token, usize, usize)], line: u32, col: u32) -> Option<(&'a Token, usize, usize)> {
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

fn instr_doc(mnemonic: &str) -> Option<&'static str> {
    Some(match mnemonic.to_uppercase().as_str() {
        "CLS" => "Clear the display.\n\n`00E0`",
        "RET" => "Return from subroutine.\n\n`00EE`",
        "JP" => "Jump to address.\n\n`1NNN` — Set PC = NNN\n\n`BNNN` — Jump to NNN + V0",
        "CALL" => "Call subroutine at address.\n\n`2NNN` — Push PC, set PC = NNN",
        "SE" => "Skip if equal.\n\n`3XKK` — Skip if Vx == KK\n`5XY0` — Skip if Vx == Vy",
        "SNE" => "Skip if not equal.\n\n`4XKK` — Skip if Vx != KK\n`9XY0` — Skip if Vx != Vy",
        "LD" => "Load value.\n\n`6XKK` — Vx = KK\n`8XY0` — Vx = Vy\n`ANNN` — I = NNN\n`FX07` — Vx = DT\n`FX0A` — Wait key, Vx = key\n`FX15` — DT = Vx\n`FX18` — ST = Vx\n`FX29` — I = sprite(Vx)\n`FX33` — BCD of Vx\n`FX55` — Save V0..Vx\n`FX65` — Load V0..Vx",
        "ADD" => "Add.\n\n`7XKK` — Vx += KK\n`8XY4` — Vx += Vy, VF = carry\n`FX1E` — I += Vx",
        "OR" => "Bitwise OR.\n\n`8XY1` — Vx |= Vy",
        "AND" => "Bitwise AND.\n\n`8XY2` — Vx &= Vy",
        "XOR" => "Bitwise XOR.\n\n`8XY3` — Vx ^= Vy",
        "SUB" => "Subtract.\n\n`8XY5` — Vx -= Vy, VF = not borrow",
        "SUBN" => "Reverse subtract.\n\n`8XY7` — Vx = Vy - Vx, VF = not borrow",
        "SHR" => "Shift right.\n\n`8XY6` — Vx >>= 1, VF = LSB",
        "SHL" => "Shift left.\n\n`8XYE` — Vx <<= 1, VF = MSB",
        "RND" => "Random.\n\n`CXKK` — Vx = random & KK",
        "DRW" => "Draw sprite.\n\n`DXYN` — Draw N-byte sprite at (Vx, Vy), VF = collision",
        "SKP" => "Skip if key pressed.\n\n`EX9E` — Skip if key Vx is pressed",
        "SKNP" => "Skip if key not pressed.\n\n`EXA1` — Skip if key Vx is not pressed",
        _ => return None,
    })
}

fn reg_doc(name: &str) -> Option<&'static str> {
    Some(match name.to_uppercase().as_str() {
        "V0" | "V1" | "V2" | "V3" | "V4" | "V5" | "V6" | "V7"
        | "V8" | "V9" | "VA" | "VB" | "VC" | "VD" | "VE" => {
            "General-purpose 8-bit register."
        }
        "VF" => "General-purpose 8-bit register. Used as carry/flag register by many instructions.",
        "I" => "16-bit address register. Used to point at memory locations.",
        "DT" => "Delay timer. Decrements at 60Hz until 0.",
        "ST" => "Sound timer. Decrements at 60Hz; sound plays while > 0.",
        _ => return None,
    })
}

fn dir_doc(name: &str) -> Option<&'static str> {
    Some(match name.to_lowercase().as_str() {
        "org" => "Set the origin (load address) for subsequent code.\n\n`.org NNNN` — Continue assembly at address NNNN",
        "byte" => "Emit literal bytes.\n\n`.byte VAL1, VAL2, ...` — Emit one or more 8-bit values",
        "word" => "Emit literal words.\n\n`.word VAL1, VAL2, ...` — Emit one or more 16-bit values (big-endian)",
        "ascii" => "Emit ASCII string (no null terminator).\n\n`.ascii \"text\"`",
        "asciz" => "Emit ASCII string with null terminator.\n\n`.asciz \"text\"`",
        "align" => "Pad with zeros to alignment boundary.\n\n`.align N` — Pad to next N-byte boundary",
        "space" => "Reserve N bytes of zero.\n\n`.space N`",
        "const" => "Define a numeric constant.\n\n`.const NAME = VALUE` — Constant usable as immediate operand",
        "struct" => "Define a structure layout.\n\n```\nstruct Name {\n  field1 byte\n  field2 word\n}\n```\nGenerates Name.field1, Name.field2, Name.SIZE constants.",
        _ => return None,
    })
}

pub fn get_hover(ws: &Workspace, uri: &Url, pos: Position) -> Option<Hover> {
    let doc = ws.get_document(uri)?;
    let tokens = doc.tokens.as_ref()?;
    let analysis = doc.analysis.as_ref()?;
    let sym = doc.symbol_table.as_ref()?;

    let (tok, _line, _col) = find_token(tokens, pos.line, pos.character)?;

    match tok {
        Token::Word(w) => {
            let upper = w.to_uppercase();

            // Check instructions
            if let Some(doc) = instr_doc(&upper) {
                return Some(Hover {
                    contents: HoverContents::Scalar(MarkedString::String(doc.to_string())),
                    range: None,
                });
            }

            // Check registers
            if let Some(doc) = reg_doc(&upper) {
                return Some(Hover {
                    contents: HoverContents::Scalar(MarkedString::String(doc.to_string())),
                    range: None,
                });
            }

            // Check directives and keywords
            if let Some(doc) = dir_doc(&upper) {
                return Some(Hover {
                    contents: HoverContents::Scalar(MarkedString::String(doc.to_string())),
                    range: None,
                });
            }

            // Check symbols (labels and constants)
            if let Some(val) = sym.resolve(w) {
                let kind = if analysis.source.contains(&format!("\n{}:", w))
                    || analysis.source.contains(&format!("{}:", w))
                    || analysis.source.contains(&format!("{} LABEL", w))
                {
                    "Label"
                } else {
                    "Constant"
                };
                return Some(Hover {
                    contents: HoverContents::Scalar(MarkedString::String(
                        format!("**{}** `{}`\n\nValue: `0x{:03X}` ({})", kind, w, val, val)
                    )),
                    range: None,
                });
            }
        }
        Token::Number(n) => {
            return Some(Hover {
                contents: HoverContents::Scalar(MarkedString::String(
                    format!("Decimal: `{}`\nHex: `0x{:X}`\n", n, n)
                )),
                range: None,
            });
        }
        _ => {}
    }

    None
}
