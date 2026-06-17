use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

fn send_msg(writer: &mut impl Write, msg: &serde_json::Value) {
    let body = serde_json::to_string(msg).unwrap();
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes()).unwrap();
    writer.write_all(body.as_bytes()).unwrap();
    writer.flush().unwrap();
}

fn read_msg(reader: &mut BufReader<impl Read>) -> serde_json::Value {
    let mut header = String::new();
    loop {
        header.clear();
        reader.read_line(&mut header).unwrap();
        if header.trim().is_empty() { continue; }
        if header.starts_with("Content-Length:") {
            let len: usize = header.trim_start_matches("Content-Length: ").trim().parse().unwrap();
            reader.read_line(&mut header).unwrap();
            let mut body = vec![0u8; len];
            reader.read_exact(&mut body).unwrap();
            return serde_json::from_slice(&body).unwrap();
        }
    }
}

fn read_notification(reader: &mut BufReader<impl Read>) -> serde_json::Value {
    loop {
        let msg = read_msg(reader);
        if msg.get("method").is_some() {
            return msg;
        }
    }
}

fn read_response(reader: &mut BufReader<impl Read>, expected_id: u64) -> serde_json::Value {
    loop {
        let msg = read_msg(reader);
        if msg.get("id") == Some(&serde_json::json!(expected_id)) {
            return msg;
        }
    }
}

#[test]
fn test_lsp_diagnostics() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_chip8-lsp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Initialize
    send_msg(&mut stdin, &serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "processId": null, "capabilities": {}, "rootUri": null }
    }));
    let resp = read_response(&mut reader, 1);
    assert!(resp["result"]["capabilities"]["hoverProvider"].as_bool().unwrap_or(false));

    // Initialized
    send_msg(&mut stdin, &serde_json::json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));
    let _log = read_notification(&mut reader);

    // Open document with errors
    send_msg(&mut stdin, &serde_json::json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///test.asm", "languageId": "chip8", "version": 1,
                "text": "start:\n    CLS\n    LD V0, 10\n    INVALID_OPCODE\n"
            }
        }
    }));

    // Read diagnostics
    let diag = read_notification(&mut reader);
    assert_eq!(diag["method"], "textDocument/publishDiagnostics");
    let diagnoses = diag["params"]["diagnostics"].as_array().unwrap();
    assert!(!diagnoses.is_empty(), "expected diagnostics");
    assert!(diagnoses[0]["message"].as_str().unwrap().contains("INVALID_OPCODE"),
        "expected INVALID_OPCODE error");

    // Hover on CLS at (1,5)
    send_msg(&mut stdin, &serde_json::json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/hover",
        "params": {
            "textDocument": { "uri": "file:///test.asm" },
            "position": { "line": 1, "character": 5 }
        }
    }));
    let resp = read_response(&mut reader, 2);
    let hover = &resp["result"];
    assert!(!hover.is_null(), "hover should not be null: {:?}", resp);

    // Shutdown
    send_msg(&mut stdin, &serde_json::json!({
        "jsonrpc": "2.0", "id": 3, "method": "shutdown", "params": null
    }));
    let _resp = read_response(&mut reader, 3);

    // Exit
    send_msg(&mut stdin, &serde_json::json!({
        "jsonrpc": "2.0", "method": "exit", "params": null
    }));

    // Wait with timeout
    let _ = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(2));
        let _ = child.kill();
    });
}
