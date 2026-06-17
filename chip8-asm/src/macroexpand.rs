use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MacroDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<String>,
}

#[derive(Debug)]
pub enum MacroError {
    RecursiveExpansion(String, Vec<String>),
    WrongArgCount {
        name: String,
        expected: usize,
        got: usize,
        line: usize,
    },
    UnclosedMacro(String),
    EmptyMacroName,
}

impl std::fmt::Display for MacroError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MacroError::RecursiveExpansion(_name, chain) => {
                write!(f, "recursive macro expansion: {}", chain.join(" → "))
            }
            MacroError::WrongArgCount {
                name, expected, got, ..
            } => {
                write!(
                    f,
                    "macro '{}' expects {} argument(s), got {}",
                    name, expected, got
                )
            }
            MacroError::UnclosedMacro(name) => {
                write!(f, "unclosed macro '{}'", name)
            }
            MacroError::EmptyMacroName => {
                write!(f, "empty macro name")
            }
        }
    }
}

// ── Phase 1: Collect macro definitions ──────────────────────────────────

pub fn collect_definitions(source: &str) -> Result<(String, Vec<MacroDef>), MacroError> {
    let lines: Vec<&str> = source.lines().collect();
    let mut remaining = Vec::new();
    let mut definitions = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if let Some(rest) = trim_macro_prefix(trimmed) {
            // rest: "name params { body..." or "name params {"
            let (name, params, body_head) = parse_header(rest)?;

            let mut body = Vec::new();

            if let Some(head) = body_head {
                // { was on this line, head is whatever came after {
                if collect_single_line_body(&head, &mut body) {
                    // body was fully on the header line — line consumed
                    i += 1;
                } else {
                    i += 1; // advance past the header line
                    read_body_lines(&lines, &mut i, &head, &mut body)?;
                }
            } else {
                // No { on header line; look for it on next non-empty line
                i += 1;
                while i < lines.len() && lines[i].trim().is_empty() {
                    remaining.push(String::new());
                    i += 1;
                }
                if i >= lines.len() {
                    return Err(MacroError::UnclosedMacro(name));
                }
                let brace_line = lines[i].trim();
                if !brace_line.starts_with('{') {
                    return Err(MacroError::UnclosedMacro(name));
                }
                let after_brace = brace_line[1..].trim().to_string();
                i += 1;
                if collect_single_line_body(&after_brace, &mut body) {
                    // entire body was on the brace line
                } else {
                    read_body_lines(&lines, &mut i, &after_brace, &mut body)?;
                }
            }

            definitions.push(MacroDef { name, params, body });
        } else {
            remaining.push(lines[i].to_string());
            i += 1;
        }
    }

    Ok((remaining.join("\n"), definitions))
}

fn trim_macro_prefix(s: &str) -> Option<&str> {
    s.strip_prefix("macro ")
        .or_else(|| s.strip_prefix("MACRO "))
        .map(|r| r.trim())
}

fn parse_header(
    rest: &str,
) -> Result<(String, Vec<String>, Option<String>), MacroError> {
    // rest: "name params { body..." or "name params"
    let (header_part, after_brace) = if let Some(pos) = rest.find('{') {
        (rest[..pos].trim(), Some(rest[pos + 1..].to_string()))
    } else {
        (rest.trim(), None)
    };

    let mut parts = header_part.splitn(2, char::is_whitespace);
    let name = parts.next().ok_or(MacroError::EmptyMacroName)?.to_string();
    if name.is_empty() {
        return Err(MacroError::EmptyMacroName);
    }

    let params_str = parts.next().unwrap_or("").trim();
    let params: Vec<String> = if params_str.is_empty() {
        Vec::new()
    } else {
        params_str.split(',').map(|s| s.trim().to_string()).collect()
    };

    Ok((name, params, after_brace))
}

fn collect_single_line_body(text: &str, body: &mut Vec<String>) -> bool {
    let trimmed = text.trim();
    if trimmed.ends_with('}') {
        let inner = trimmed[..trimmed.len() - 1].trim();
        if !inner.is_empty() {
            body.push(inner.to_string());
        }
        true
    } else if !trimmed.is_empty() {
        body.push(trimmed.to_string());
        false
    } else {
        // text after { is empty
        false
    }
}

fn read_body_lines(
    lines: &[&str],
    i: &mut usize,
    head: &str,
    body: &mut Vec<String>,
) -> Result<(), MacroError> {
    if !head.is_empty() {
        body.push(head.to_string());
    }
    loop {
        if *i >= lines.len() {
            return Err(MacroError::UnclosedMacro("missing '}'".into()));
        }
        let bl = lines[*i].trim();
        if bl == "}" {
            *i += 1;
            break;
        }
        if bl.ends_with('}') {
            let inner = bl[..bl.len() - 1].trim();
            if !inner.is_empty() {
                body.push(inner.to_string());
            }
            *i += 1;
            break;
        }
        body.push(lines[*i].to_string());
        *i += 1;
    }
    Ok(())
}

// ── Phase 2: Expand macro invocations ───────────────────────────────────

pub fn expand(source: &str, macros: &[MacroDef]) -> Result<String, MacroError> {
    let macro_map: HashMap<String, &MacroDef> =
        macros.iter().map(|m| (m.name.clone(), m)).collect();

    if macro_map.is_empty() {
        return Ok(source.to_string());
    }

    let lines: Vec<&str> = source.lines().collect();
    let mut output = String::new();
    let mut counter: u64 = 0;

    for line_idx in 0..lines.len() {
        let expanded = expand_one_line(
            lines[line_idx],
            &macro_map,
            &mut counter,
            line_idx,
            &mut Vec::new(),
        )?;
        output.push_str(&expanded);
        output.push('\n');
    }

    Ok(output)
}

fn expand_one_line(
    line: &str,
    macro_map: &HashMap<String, &MacroDef>,
    counter: &mut u64,
    line_idx: usize,
    expansion_stack: &mut Vec<String>,
) -> Result<String, MacroError> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
        return Ok(line.to_string());
    }

    let (first_word, rest) = split_first_word(trimmed);
    let mac = match macro_map.get(first_word) {
        Some(m) => m,
        None => return Ok(line.to_string()),
    };

    let args = parse_invocation_args(rest);
    if args.len() != mac.params.len() {
        return Err(MacroError::WrongArgCount {
            name: mac.name.clone(),
            expected: mac.params.len(),
            got: args.len(),
            line: line_idx,
        });
    }

    if expansion_stack.contains(&mac.name) {
        let cycle: Vec<String> = expansion_stack
            .iter()
            .cloned()
            .chain(std::iter::once(mac.name.clone()))
            .collect();
        return Err(MacroError::RecursiveExpansion(mac.name.clone(), cycle));
    }

    expansion_stack.push(mac.name.clone());

    let subst: HashMap<String, String> = mac
        .params
        .iter()
        .zip(args.iter().cloned())
        .map(|(k, v)| (k.clone(), v))
        .collect();

    *counter += 1;
    let invocation_id = *counter;

    let indent_len = line.len() - trimmed.len();
    let indent = &line[..indent_len];

    let mut expanded_lines = Vec::new();
    for body_line in &mac.body {
        let subbed = substitute_line(body_line, &subst, invocation_id);
        let inner =
            expand_one_line(&subbed, macro_map, counter, line_idx, expansion_stack)?;
        for inner_line in inner.lines() {
            if inner_line.is_empty() {
                expanded_lines.push(String::new());
            } else {
                expanded_lines.push(format!("{}{}", indent, inner_line));
            }
        }
    }

    expansion_stack.pop();

    Ok(expanded_lines.join("\n"))
}

fn split_first_word(s: &str) -> (&str, &str) {
    let s = s.trim();
    let word_end = s
        .find(|c: char| c.is_whitespace() || c == ',')
        .unwrap_or(s.len());
    let word = &s[..word_end];
    let rest = s[word_end..].trim();
    (word, rest)
}

fn parse_invocation_args(rest: &str) -> Vec<String> {
    if rest.is_empty() {
        return Vec::new();
    }

    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_string = false;

    for c in rest.chars() {
        match c {
            '"' => {
                in_string = !in_string;
                current.push(c);
            }
            ',' if !in_string => {
                let a = current.trim().to_string();
                if !a.is_empty() {
                    args.push(a);
                }
                current = String::new();
            }
            _ => current.push(c),
        }
    }

    let a = current.trim().to_string();
    if !a.is_empty() {
        args.push(a);
    }

    args
}

fn substitute_line(line: &str, subst: &HashMap<String, String>, invocation_id: u64) -> String {
    let mut result = String::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '%' && i + 1 < chars.len() && chars[i + 1] == '%' {
            let start = i + 2;
            let mut end = start;
            while end < chars.len()
                && (chars[end].is_ascii_alphanumeric() || chars[end] == '_')
            {
                end += 1;
            }
            let label: String = chars[start..end].iter().collect();
            result.push_str(&format!("__m{}_{}", invocation_id, label));
            i = end;
        } else if chars[i].is_ascii_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric() || chars[i] == '_')
            {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            if let Some(replacement) = subst.get(&word) {
                result.push_str(replacement);
            } else {
                result.push_str(&word);
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}
