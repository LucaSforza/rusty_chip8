use tower_lsp::lsp_types::*;

use crate::workspace::Workspace;

const INSTRUCTIONS: &[&str] = &[
    "CLS", "RET", "SYS", "JP ", "CALL ", "SE ", "SNE ", "LD ", "ADD ",
    "OR ", "AND ", "XOR ", "SUB ", "SUBN ", "SHR ", "SHL ", "RND ", "DRW ",
    "SKP ", "SKNP ",
];

const REGISTERS: &[&str] = &[
    "V0", "V1", "V2", "V3", "V4", "V5", "V6", "V7",
    "V8", "V9", "VA", "VB", "VC", "VD", "VE", "VF",
    "I", "DT", "ST", "K", "F", "B",
];

const DIRECTIVES: &[&str] = &[
    ".org ", ".byte ", ".word ", ".ascii ", ".asciz ",
    ".align ", ".space ", ".const ", ".struct ",
];

fn text(kind: CompletionItemKind, label: &str, detail: &str, insert: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        detail: Some(detail.to_string()),
        insert_text: Some(insert.to_string()),
        ..Default::default()
    }
}

pub fn get_completions(ws: &Workspace, uri: &Url, pos: Position) -> Option<CompletionResponse> {
    let doc = ws.get_document(uri)?;
    let source = &doc.source;

    let lines: Vec<&str> = source.lines().collect();
    let current_line = lines.get(pos.line as usize).unwrap_or(&"");
    let line_prefix = &current_line[..(pos.character as usize).min(current_line.len())];

    let mut items: Vec<CompletionItem> = Vec::new();

    let trimmed = line_prefix.trim_start();

    // After a dot: only directives
    if trimmed.starts_with('.') {
        for dir in DIRECTIVES {
            items.push(text(CompletionItemKind::KEYWORD, dir.trim_end(), "Directive", dir));
        }
        return Some(CompletionResponse::Array(items));
    }

    // At line start or after instruction: offer instructions + labels + macro names
    if trimmed.is_empty() || trimmed.chars().all(|c| c.is_uppercase() || c.is_whitespace()) {
        for &instr in INSTRUCTIONS {
            let detail = match instr.trim() {
                "CLS" => "Clear display",
                "RET" => "Return from subroutine",
                "JP" => "Jump to address",
                "CALL" => "Call subroutine",
                "SE" => "Skip if equal",
                "SNE" => "Skip if not equal",
                "LD" => "Load value",
                "ADD" => "Add",
                "OR" => "Bitwise OR",
                "AND" => "Bitwise AND",
                "XOR" => "Bitwise XOR",
                "SUB" => "Subtract",
                "SUBN" => "Reverse subtract",
                "SHR" => "Shift right",
                "SHL" => "Shift left",
                "RND" => "Random AND",
                "DRW" => "Draw sprite",
                "SKP" => "Skip if key pressed",
                "SKNP" => "Skip if key not pressed",
                _ => "Instruction",
            };
            items.push(text(CompletionItemKind::OPERATOR, instr.trim(), detail, instr));
        }

        // Labels from symbol table
        if let Some(analysis) = &doc.analysis {
            for (name, addr) in analysis.symbol_table.labels() {
                items.push(text(
                    CompletionItemKind::FUNCTION,
                    name,
                    &format!("Label — 0x{:03X}", addr),
                    name,
                ));
            }
            for (name, val) in analysis.symbol_table.constants() {
                if !name.contains('.') {
                    items.push(text(
                        CompletionItemKind::PROPERTY,
                        name,
                        &format!("Constant = {}", val),
                        name,
                    ));
                }
            }
        }

        // Directives (with leading dot)
        for dir in DIRECTIVES {
            items.push(text(CompletionItemKind::KEYWORD, dir.trim_end(), "Directive", dir));
        }
    }

    // Registers always available
    for reg in REGISTERS {
        items.push(text(CompletionItemKind::VARIABLE, reg, "Register", reg));
    }

    Some(CompletionResponse::Array(items))
}
