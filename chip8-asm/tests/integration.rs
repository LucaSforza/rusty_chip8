use std::collections::HashMap;
use std::path::PathBuf;
use chip8_asm::{assemble, assemble_with, AssemblyOptions};

fn asm(src: &str) -> Vec<u8> {
    assemble(src).unwrap().bytes
}

fn asm_with(src: &str, files: HashMap<&str, &str>) -> Vec<u8> {
    let files: HashMap<PathBuf, String> = files
        .into_iter()
        .map(|(k, v)| (PathBuf::from(k), v.to_string()))
        .collect();
    assemble_with(
        src,
        &AssemblyOptions {
            files,
            ..Default::default()
        },
    )
    .unwrap()
    .bytes
}

fn must_err(src: &str) -> Vec<chip8_asm::AssemblyError> {
    assemble(src).unwrap_err()
}

fn must_err_with(
    src: &str,
    files: HashMap<&str, &str>,
) -> Vec<chip8_asm::AssemblyError> {
    let files: HashMap<PathBuf, String> = files
        .into_iter()
        .map(|(k, v)| (PathBuf::from(k), v.to_string()))
        .collect();
    assemble_with(
        src,
        &AssemblyOptions {
            files,
            ..Default::default()
        },
    )
    .unwrap_err()
}

// ── Test 1: Include ─────────────────────────────────────────────────────

#[test]
fn test_include_basic() {
    let bytes = asm_with(
        "include \"constants.asm\"\nLD V0, TEN\n",
        HashMap::from([("constants.asm", ".const TEN = 10\n")]),
    );
    assert_eq!(bytes, vec![0x60, 0x0A]);
}

#[test]
fn test_include_nested() {
    let bytes = asm_with(
        "include \"a.asm\"\nLD V0, NUM\n",
        HashMap::from([
            ("a.asm", "include \"b.asm\"\n"),
            ("b.asm", ".const NUM = 42\n"),
        ]),
    );
    assert_eq!(bytes, vec![0x60, 0x2A]);
}

#[test]
fn test_include_cycle_detected() {
    let errs = must_err_with(
        "include \"a.asm\"\n",
        HashMap::from([("a.asm", "include \"a.asm\"\n")]),
    );
    let msg = errs.first().unwrap().to_string();
    assert!(
        msg.contains("cycle") || msg.contains("Cycle"),
        "expected cycle error, got: {msg}"
    );
}

#[test]
fn test_include_duplicate_allowed() {
    let bytes = asm_with(
        "include \"lib.asm\"\ninclude \"lib.asm\"\n",
        HashMap::from([("lib.asm", "CLS\n")]),
    );
    assert_eq!(bytes, vec![0x00, 0xE0, 0x00, 0xE0]);
}

// ── Test 2: Simple Macro ────────────────────────────────────────────────

#[test]
fn test_simple_macro() {
    let src = r#"
macro clear_screen {
CLS
}

clear_screen
"#;
    let bytes = asm(src);
    assert_eq!(bytes, vec![0x00, 0xE0]);
}

// ── Test 3: Parameter Macro ─────────────────────────────────────────────

#[test]
fn test_parameter_macro() {
    let src = r#"
macro load reg, value {
LD reg, value
}

load V0, 10
"#;
    let bytes = asm(src);
    assert_eq!(bytes, vec![0x60, 0x0A]);
}

// ── Test 4: Multiple Macro Calls ────────────────────────────────────────

#[test]
fn test_multiple_macro_calls() {
    let src = r#"
macro ldv reg, val {
LD reg, val
}

ldv V0, 1
ldv V1, 2
"#;
    let bytes = asm(src);
    assert_eq!(bytes, vec![0x60, 0x01, 0x61, 0x02]);
}

// ── Test 5: Macro Local Labels ──────────────────────────────────────────

#[test]
fn test_macro_local_labels() {
    let src = r#"
macro spin reg {
%%loop:
ADD reg, 1
JP %%loop
}

spin V0
spin V1
"#;
    let bytes = asm(src);
    assert_eq!(bytes.len(), 8);
    // spin V0: ADD V0,1 = 70 01, JP __m1_loop = 1nnn
    assert_eq!(bytes[0], 0x70);
    assert_eq!(bytes[1], 0x01);
    assert_eq!(bytes[2] & 0xF0, 0x10);
    // spin V1: ADD V1,1 = 71 01, JP __m2_loop = 1nnn
    assert_eq!(bytes[4], 0x71);
    assert_eq!(bytes[5], 0x01);
    assert_eq!(bytes[6] & 0xF0, 0x10);
    // Verify the two JP targets differ (different local labels)
    let jp1_addr = ((bytes[2] as u16) << 8 | bytes[3] as u16) & 0x0FFF;
    let jp2_addr = ((bytes[6] as u16) << 8 | bytes[7] as u16) & 0x0FFF;
    assert_ne!(jp1_addr, jp2_addr, "local labels should be unique");
}

// ── Test 6: Structure Offsets ───────────────────────────────────────────

#[test]
fn test_structure_offsets() {
    let src = r#"
struct Sprite {
x byte
y byte
width byte
height byte
}

.word Sprite.width
.word Sprite.SIZE
"#;
    let bytes = asm(src);
    assert_eq!(bytes, vec![0x00, 0x02, 0x00, 0x04]);
}

#[test]
fn test_structure_with_word_field() {
    let src = r#"
struct Point {
x byte
y byte
addr word
}
.word Point.addr
"#;
    let bytes = asm(src);
    assert_eq!(bytes, vec![0x00, 0x02]);
}

// ── Test 7: Include + Macro ─────────────────────────────────────────────

#[test]
fn test_include_plus_macro() {
    let bytes = asm_with(
        "include \"lib.asm\"\nloadzero V3\n",
        HashMap::from([("lib.asm", "macro loadzero reg {\nLD reg, 0\n}\n")]),
    );
    assert_eq!(bytes, vec![0x63, 0x00]);
}

// ── Test 8: Recursive Macro Detection ───────────────────────────────────

#[test]
fn test_recursive_macro_detected() {
    let src = r#"
macro A {
A
}

A
"#;
    let errs = must_err(src);
    let msg = errs.first().unwrap().to_string();
    assert!(
        msg.contains("recursive") || msg.contains("Recursive"),
        "expected recursion error, got: {msg}"
    );
}
