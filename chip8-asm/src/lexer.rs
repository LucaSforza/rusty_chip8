#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),
    Number(u16),
    String(String),
    Colon,
    Comma,
    Dot,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Equals,
    Newline,
    Eof,
    Error(String),
}

pub fn tokenize(source: &str) -> Vec<(Token, usize, usize)> {
    let mut tokens = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
        let line_tokens = lex_line(line, line_num);
        if !line_tokens.is_empty() {
            tokens.extend(line_tokens);
        }
        tokens.push((Token::Newline, line_num, line.len().max(1) - 1));
    }

    if tokens.last().map(|t| &t.0) != Some(&Token::Newline) {
        let last_line = lines.len().max(1) - 1;
        tokens.push((Token::Newline, last_line, 0));
    }
    tokens.push((Token::Eof, lines.len().max(1) - 1, 0));

    tokens
}

fn lex_line(line: &str, line_num: usize) -> Vec<(Token, usize, usize)> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }

        // Comment: ; or # (unless # is immediate prefix)
        if chars[i] == ';' {
            break;
        }
        if chars[i] == '#' {
            if i + 1 < chars.len()
                && (chars[i + 1] == '$'
                    || chars[i + 1].is_ascii_digit()
                    || (chars[i + 1] == '0' && i + 2 < chars.len() && chars[i + 2] == 'x'))
            {
                i += 1;
                continue;
            }
            break;
        }

        // Hex: $FF or 0xFF
        if chars[i] == '$'
            || (chars[i] == '0' && i + 1 < chars.len() && chars[i + 1] == 'x')
        {
            let start = if chars[i] == '$' { i + 1 } else { i + 2 };
            let mut end = start;
            while end < chars.len() && chars[end].is_ascii_hexdigit() {
                end += 1;
            }
            if end == start {
                let col = i;
                tokens.push((
                    Token::Error("expected hex digits after '$' or '0x'".into()),
                    line_num,
                    col,
                ));
                i = end.max(i + 1);
                continue;
            }
            match u16::from_str_radix(&line[start..end], 16) {
                Ok(val) => tokens.push((Token::Number(val), line_num, start)),
                Err(e) => {
                    tokens.push((Token::Error(format!("invalid hex number: {}", e)), line_num, start));
                }
            }
            i = end;
            continue;
        }

        // Decimal number
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            match line[start..i].parse::<u16>() {
                Ok(val) => tokens.push((Token::Number(val), line_num, start)),
                Err(e) => {
                    tokens.push((Token::Error(format!("invalid number: {}", e)), line_num, start));
                }
            }
            continue;
        }

        // String literal
        if chars[i] == '"' {
            let start = i + 1;
            let mut end = start;
            while end < chars.len() && chars[end] != '"' {
                end += 1;
            }
            if end >= chars.len() {
                tokens.push((Token::Error("unterminated string literal".into()), line_num, start));
                i = end;
                continue;
            }
            tokens.push((Token::String(line[start..end].to_string()), line_num, start));
            i = end + 1;
            continue;
        }

        // Word (identifier, mnemonic, register, directive name, dotted struct field)
        if chars[i].is_ascii_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric()
                    || chars[i] == '_'
                    || (chars[i] == '.'
                        && i + 1 < chars.len()
                        && (chars[i + 1].is_ascii_alphanumeric()
                            || chars[i + 1] == '_')))
            {
                i += 1;
            }
            tokens.push((Token::Word(line[start..i].to_string()), line_num, start));
            continue;
        }

        // Single-char punctuation
        let col = i;
        match chars[i] {
            ':' => tokens.push((Token::Colon, line_num, col)),
            ',' => tokens.push((Token::Comma, line_num, col)),
            '.' => tokens.push((Token::Dot, line_num, col)),
            '[' => tokens.push((Token::LBracket, line_num, col)),
            ']' => tokens.push((Token::RBracket, line_num, col)),
            '{' => tokens.push((Token::LBrace, line_num, col)),
            '}' => tokens.push((Token::RBrace, line_num, col)),
            '=' => tokens.push((Token::Equals, line_num, col)),
            c => {
                tokens.push((
                    Token::Error(format!("unexpected character '{}'", c)),
                    line_num,
                    col,
                ));
            }
        }
        i += 1;
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_numbers(tokens: &[(Token, usize, usize)]) -> Vec<u16> {
        tokens
            .iter()
            .filter_map(|(t, _, _)| {
                if let Token::Number(n) = t {
                    Some(*n)
                } else {
                    None
                }
            })
            .collect()
    }

    #[test]
    fn test_hex_dollar() {
        let t = tokenize("LD V0, #$0F");
        assert!(collect_numbers(&t).contains(&0x0F));
    }

    #[test]
    fn test_hex_0x() {
        let t = tokenize("JP #0x200");
        assert!(collect_numbers(&t).contains(&0x200));
    }

    #[test]
    fn test_decimal() {
        let t = tokenize("ADD V0, 15");
        assert!(collect_numbers(&t).contains(&15));
    }

    #[test]
    fn test_semicolon_comment() {
        let t = tokenize("CLS ; clear screen\nRET");
        let words: Vec<&str> = t
            .iter()
            .filter_map(|(tok, _, _)| match tok {
                Token::Word(w) => Some(w.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(words, vec!["CLS", "RET"]);
    }

    #[test]
    fn test_hash_comment() {
        let t = tokenize("CLS # clear screen\nRET");
        let words: Vec<&str> = t
            .iter()
            .filter_map(|(tok, _, _)| match tok {
                Token::Word(w) => Some(w.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(words, vec!["CLS", "RET"]);
    }

    #[test]
    fn test_label() {
        let t = tokenize("start:\nCLS");
        let words: Vec<&str> = t
            .iter()
            .filter_map(|(tok, _, _)| match tok {
                Token::Word(w) => Some(w.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(words, vec!["start", "CLS"]);
    }

    #[test]
    fn test_brackets() {
        let t = tokenize("LD [I], V0");
        assert!(t.iter().any(|(tok, _, _)| matches!(tok, Token::LBracket)));
        assert!(t.iter().any(|(tok, _, _)| matches!(tok, Token::RBracket)));
    }

    #[test]
    fn test_directive_dot() {
        let t = tokenize(".org 0x200");
        assert!(t.iter().any(|(tok, _, _)| matches!(tok, Token::Dot)));
        assert!(t.iter().any(|(tok, _, _)| matches!(tok, Token::Word(w) if w == "org")));
    }

    #[test]
    fn test_hash_not_comment_when_immediate() {
        let t = tokenize("LD V0, #$FF");
        assert!(collect_numbers(&t).contains(&0xFF));
    }

    #[test]
    fn test_hash_comment_when_space() {
        let t = tokenize("CLS\n# this is a comment\nRET");
        let words: Vec<&str> = t
            .iter()
            .filter_map(|(tok, _, _)| match tok {
                Token::Word(w) => Some(w.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(words, vec!["CLS", "RET"]);
    }
}
