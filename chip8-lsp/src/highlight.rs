use tower_lsp::lsp_types::*;

pub const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::OPERATOR,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::KEYWORD,
    SemanticTokenType::FUNCTION,
    SemanticTokenType::PROPERTY,
    SemanticTokenType::NAMESPACE,
    SemanticTokenType::NUMBER,
    SemanticTokenType::STRING,
    SemanticTokenType::COMMENT,
];

pub const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION,
    SemanticTokenModifier::READONLY,
    SemanticTokenModifier::MODIFICATION,
    SemanticTokenModifier::DEFAULT_LIBRARY,
];

fn token_type_index(t: &SemanticTokenType) -> u32 {
    TOKEN_TYPES.iter().position(|x| x == t).unwrap_or(0) as u32
}

fn token_mod_bits(mods: &[SemanticTokenModifier]) -> u32 {
    let mut bits = 0u32;
    for m in mods {
        if let Some(i) = TOKEN_MODIFIERS.iter().position(|x| x == m) {
            bits |= 1 << i;
        }
    }
    bits
}

use crate::workspace::Workspace;
use chip8_asm::lexer::Token;

fn classify_token(word: &str) -> (SemanticTokenType, Vec<SemanticTokenModifier>) {
    let upper = word.to_uppercase();
    match upper.as_str() {
        "CLS" | "RET" | "SYS" | "JP" | "CALL" | "SE" | "SNE" | "LD" | "ADD" | "OR"
        | "AND" | "XOR" | "SUB" | "SUBN" | "SHR" | "SHL" | "RND" | "DRW" | "SKP"
        | "SKNP" => (SemanticTokenType::OPERATOR, vec![]),

        "V0" | "V1" | "V2" | "V3" | "V4" | "V5" | "V6" | "V7"
        | "V8" | "V9" | "VA" | "VB" | "VC" | "VD" | "VE" | "VF" => {
            (SemanticTokenType::VARIABLE, vec![])
        }

        "I" => (SemanticTokenType::VARIABLE, vec![SemanticTokenModifier::READONLY]),
        "DT" | "ST" => (SemanticTokenType::VARIABLE, vec![SemanticTokenModifier::READONLY]),
        "K" | "F" | "B" => (SemanticTokenType::VARIABLE, vec![]),

        "ORG" | "BYTE" | "WORD" | "ASCII" | "ASCIZ" | "ALIGN" | "SPACE" => {
            (SemanticTokenType::KEYWORD, vec![])
        }

        "CONST" => (SemanticTokenType::KEYWORD, vec![SemanticTokenModifier::MODIFICATION]),
        "STRUCT" => (SemanticTokenType::KEYWORD, vec![SemanticTokenModifier::DECLARATION]),
        "MACRO" | "INCLUDE" => (SemanticTokenType::KEYWORD, vec![]),

        _ => (SemanticTokenType::FUNCTION, vec![]),
    }
}

pub fn get_semantic_tokens(ws: &Workspace, uri: &Url) -> Option<SemanticTokensResult> {
    let doc = ws.get_document(uri)?;
    let tokens = doc.tokens.as_ref()?;
    let _analysis = doc.analysis.as_ref()?;

    let mut data: Vec<SemanticToken> = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_char = 0u32;

    for (token, line, col) in tokens {
        match token {
            Token::Word(w) => {
                let (ttype, tmods) = classify_token(w);
                let delta_line = *line as u32 - prev_line;
                let delta_char = if delta_line == 0 {
                    *col as u32 - prev_char
                } else {
                    *col as u32
                };
                let length = w.len() as u32;
                let token_type = token_type_index(&ttype);
                let token_modifiers_bitset = token_mod_bits(&tmods);
                data.push(SemanticToken { delta_line, delta_start: delta_char, length, token_type, token_modifiers_bitset });
                prev_line = *line as u32;
                prev_char = *col as u32 + length;
            }
            Token::Number(_n) => {
                let delta_line = *line as u32 - prev_line;
                let delta_char = if delta_line == 0 {
                    *col as u32 - prev_char
                } else {
                    *col as u32
                };
                let length = 4u32;
                let token_type = token_type_index(&SemanticTokenType::NUMBER);
                data.push(SemanticToken { delta_line, delta_start: delta_char, length, token_type, token_modifiers_bitset: 0 });
                prev_line = *line as u32;
                prev_char = *col as u32 + length;
            }
            Token::String(s) => {
                let delta_line = *line as u32 - prev_line;
                let delta_char = if delta_line == 0 {
                    *col as u32 - prev_char
                } else {
                    *col as u32
                };
                let length = (s.len() + 2) as u32;
                let token_type = token_type_index(&SemanticTokenType::STRING);
                data.push(SemanticToken { delta_line, delta_start: delta_char, length, token_type, token_modifiers_bitset: 0 });
                prev_line = *line as u32;
                prev_char = *col as u32 + length;
            }
            _ => {}
        }
    }

    Some(SemanticTokensResult::Tokens(SemanticTokens { data, result_id: None }))
}
