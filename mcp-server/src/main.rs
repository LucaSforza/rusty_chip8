use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::transport::stdio;
use rmcp::{ErrorData as McpError, ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const ERR_MSG: &str = "Emulatore non in esecuzione. Avvia con: `cargo run -- <path_to_rom>`";

// ----- input structs for tools with parameters -----

#[derive(Debug, Deserialize, JsonSchema)]
struct MemoryRange {
    /// Start address (hex or decimal)
    start: u16,
    /// End address (exclusive, hex or decimal)
    end: u16,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AddressParam {
    /// Address for breakpoint
    address: u16,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct KeyParam {
    /// Hex key value (0x0-0xF). CHIP-8 hex keyboard layout:
    /// 1 2 3 C, 4 5 6 D, 7 8 9 E, A 0 B F
    key: u8,
}

// ----- MCP server state -----

#[derive(Clone)]
struct Chip8Debug {
    port: u16,
}

impl Chip8Debug {
    /// Opens fresh TCP connection per call. On failure returns McpError.
    async fn send_cmd(&self, cmd: serde_json::Value) -> Result<serde_json::Value, McpError> {
        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", self.port))
            .await
            .map_err(|_| McpError::internal_error(ERR_MSG, None))?;

        let mut line = serde_json::to_string(&cmd)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        line.push('\n');
        stream
            .write_all(line.as_bytes())
            .await
            .map_err(|_| McpError::internal_error(ERR_MSG, None))?;

        let mut reader = BufReader::new(&mut stream);
        let mut resp = String::new();
        reader
            .read_line(&mut resp)
            .await
            .map_err(|_| McpError::internal_error(ERR_MSG, None))?;

        serde_json::from_str(resp.trim())
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    fn render_screen(pixels: &[Vec<bool>]) -> String {
        let mut out = String::with_capacity(64 * 33 + 4);
        out.push('┌');
        for _ in 0..64 {
            out.push('─');
        }
        out.push_str("┐\n");
        for row in pixels.iter().take(32) {
            out.push('│');
            for &pixel in row.iter().take(64) {
                if pixel {
                    out.push('█');
                } else {
                    out.push(' ');
                }
            }
            out.push_str("│\n");
        }
        out.push('└');
        for _ in 0..64 {
            out.push('─');
        }
        out.push('┘');
        out
    }

    fn format_regs(v: &[u8; 16], i: u16, pc: u16, stack: &[u16], dt: u8, st: u8) -> String {
        let mut out = format!("PC=0x{pc:03X}  I=0x{i:03X}  DT={dt}  ST={st}");
        out.push_str("\nStack: [");
        for (idx, &val) in stack.iter().enumerate() {
            if idx > 0 {
                out.push_str(", ");
            }
            out.push_str(&format!("0x{val:03X}"));
        }
        out.push(']');
        out.push_str("\n\nRegisters:\n");
        for (idx, &val) in v.iter().enumerate() {
            let name = if idx == 0xF {
                "VF".to_string()
            } else {
                format!("V{idx:X}")
            };
            out.push_str(&format!("  {name}=0x{val:02X}"));
            if (idx + 1) % 4 == 0 {
                out.push('\n');
            }
        }
        out
    }

    fn format_memory(data: &[u8], start: u16) -> String {
        let mut out = String::new();
        for (i, chunk) in data.chunks(16).enumerate() {
            let addr = start.wrapping_add((i as u16) * 16);
            out.push_str(&format!("{addr:04X}: "));
            for &b in chunk {
                out.push_str(&format!("{b:02X} "));
            }
            out.push('\n');
        }
        out
    }

    fn diff_screens(before: &[Vec<bool>], after: &[Vec<bool>]) -> (Vec<(usize, usize, bool)>, String, String) {
        let mut changes = Vec::new();
        let mut min_x = 64usize; let mut max_x = 0usize;
        let mut min_y = 32usize; let mut max_y = 0usize;

        for y in 0..32 {
            for x in 0..64 {
                if before[y][x] != after[y][x] {
                    changes.push((x, y, after[y][x]));
                    if x < min_x { min_x = x; }
                    if x > max_x { max_x = x; }
                    if y < min_y { min_y = y; }
                    if y > max_y { max_y = y; }
                }
            }
        }

        // Mini-map: bounding box of changes
        let mut mini = String::new();
        if !changes.is_empty() {
            mini.push_str(&format!("Bbox {}-{} x {}-{}:\n", min_x, max_x, min_y, max_y));
            for y in min_y..=max_y {
                mini.push_str(&format!("{:2}|", y));
                for x in min_x..=max_x {
                    mini.push(if after[y][x] { '█' } else { if before[y][x] { '·' } else { ' ' } });
                }
                mini.push('\n');
            }
        }

        // Full screen with highlights
        let mut highlighted = String::with_capacity(64 * 33 + 4);
        highlighted.push('┌');
        for _ in 0..64 { highlighted.push('─'); }
        highlighted.push_str("┐\n");
        for y in 0..32 {
            highlighted.push('│');
            for x in 0..64 {
                if before[y][x] != after[y][x] {
                    highlighted.push(if after[y][x] { '@' } else { '·' });
                } else {
                    highlighted.push(if before[y][x] { '█' } else { ' ' });
                }
            }
            highlighted.push_str("│\n");
        }
        highlighted.push('└');
        for _ in 0..64 { highlighted.push('─'); }
        highlighted.push('┘');

        (changes, mini, highlighted)
    }
}

// ----- tool definitions -----

#[tool_router]
impl Chip8Debug {
    #[tool(description = "Render CHIP-8 display as ASCII art (64x32). On=`█` Off=` `")]
    async fn get_screen(&self) -> Result<CallToolResult, McpError> {
        let resp = self.send_cmd(json!({"cmd": "get_screen"})).await?;
        let pixels: Vec<Vec<bool>> =
            serde_json::from_value(resp["pixels"].clone()).unwrap_or_default();
        let screen = Self::render_screen(&pixels);
        Ok(CallToolResult::success(vec![Content::text(format!(
            "```\n{screen}\n```"
        ))]))
    }

    #[tool(
        description = "Dump all registers: V0-VF, I, PC, stack, delay timer, sound timer"
    )]
    async fn get_registers(&self) -> Result<CallToolResult, McpError> {
        let resp = self.send_cmd(json!({"cmd": "get_registers"})).await?;
        let v: [u8; 16] = serde_json::from_value(resp["v_regs"].clone()).unwrap_or([0; 16]);
        let i = resp["i"].as_u64().unwrap_or(0) as u16;
        let pc = resp["pc"].as_u64().unwrap_or(0) as u16;
        let stack: Vec<u16> = serde_json::from_value(resp["stack"].clone()).unwrap_or_default();
        let dt = resp["delay"].as_u64().unwrap_or(0) as u8;
        let st = resp["sound"].as_u64().unwrap_or(0) as u8;
        let formatted = Self::format_regs(&v, i, pc, &stack, dt, st);
        Ok(CallToolResult::success(vec![Content::text(formatted)]))
    }

    #[tool(description = "Read memory range")]
    async fn get_memory(
        &self,
        Parameters(MemoryRange { start, end }): Parameters<MemoryRange>,
    ) -> Result<CallToolResult, McpError> {
        let resp = self
            .send_cmd(json!({"cmd": "get_memory", "s": start, "e": end}))
            .await?;
        let data: Vec<u8> = serde_json::from_value(resp["data"].clone()).unwrap_or_default();
        let formatted = Self::format_memory(&data, start);
        Ok(CallToolResult::success(vec![Content::text(formatted)]))
    }

    #[tool(description = "Set breakpoint at memory address")]
    async fn set_breakpoint(
        &self,
        Parameters(AddressParam { address }): Parameters<AddressParam>,
    ) -> Result<CallToolResult, McpError> {
        self.send_cmd(json!({"cmd": "set_bp", "a": address}))
            .await?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Breakpoint set at 0x{address:03X}"
        ))]))
    }

    #[tool(description = "Clear breakpoint at memory address")]
    async fn clear_breakpoint(
        &self,
        Parameters(AddressParam { address }): Parameters<AddressParam>,
    ) -> Result<CallToolResult, McpError> {
        self.send_cmd(json!({"cmd": "clear_bp", "a": address}))
            .await?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Breakpoint cleared at 0x{address:03X}"
        ))]))
    }

    #[tool(description = "Execute a single instruction, then pause again")]
    async fn step(&self) -> Result<CallToolResult, McpError> {
        self.send_cmd(json!({"cmd": "step"})).await?;
        Ok(CallToolResult::success(vec![Content::text(
            "Stepped 1 instruction",
        )]))
    }

    #[tool(description = "Pause emulator execution")]
    async fn pause(&self) -> Result<CallToolResult, McpError> {
        self.send_cmd(json!({"cmd": "pause"})).await?;
        Ok(CallToolResult::success(vec![Content::text("Paused")]))
    }

    #[tool(description = "Resume emulator execution")]
    async fn resume(&self) -> Result<CallToolResult, McpError> {
        self.send_cmd(json!({"cmd": "continue"})).await?;
        Ok(CallToolResult::success(vec![Content::text("Continued")]))
    }

    #[tool(description = "Stop the emulator process")]
    async fn stop(&self) -> Result<CallToolResult, McpError> {
        self.send_cmd(json!({"cmd": "stop"})).await?;
        Ok(CallToolResult::success(vec![Content::text("Stopped")]))
    }

    #[tool(
        description = "Get full state: screen + registers + first 256 bytes of memory"
    )]
    async fn get_state(&self) -> Result<CallToolResult, McpError> {
        let resp = self.send_cmd(json!({"cmd": "get_state"})).await?;
        let pixels: Vec<Vec<bool>> =
            serde_json::from_value(resp["pixels"].clone()).unwrap_or_default();
        let v: [u8; 16] = serde_json::from_value(resp["v"].clone()).unwrap_or([0; 16]);
        let i = resp["i"].as_u64().unwrap_or(0) as u16;
        let pc = resp["pc"].as_u64().unwrap_or(0) as u16;
        let stack: Vec<u16> = serde_json::from_value(resp["stack"].clone()).unwrap_or_default();
        let dt = resp["dt"].as_u64().unwrap_or(0) as u8;
        let st = resp["st"].as_u64().unwrap_or(0) as u8;
        let mem: Vec<u8> = serde_json::from_value(resp["memory"].clone()).unwrap_or_default();

        let screen = Self::render_screen(&pixels);
        let regs = Self::format_regs(&v, i, pc, &stack, dt, st);
        let mem_hex = Self::format_memory(&mem, 0);

        let out = format!(
            "## Screen\n```\n{screen}\n```\n\n## Registers\n{regs}\n\n## Memory (0x000-0x0FF)\n{mem_hex}"
        );
        Ok(CallToolResult::success(vec![Content::text(out)]))
    }

    #[tool(description = "Press and hold a CHIP-8 hex key (0x0-0xF). Use key_release to release.")]
    async fn key_press(
        &self,
        Parameters(KeyParam { key }): Parameters<KeyParam>,
    ) -> Result<CallToolResult, McpError> {
        if key > 0x0F {
            return Err(McpError::invalid_params("key must be 0x0-0xF", None));
        }
        self.send_cmd(json!({"cmd": "key_press", "key": key}))
            .await?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Key 0x{key:X} pressed"
        ))]))
    }

    #[tool(description = "Release a previously pressed CHIP-8 hex key (0x0-0xF).")]
    async fn key_release(
        &self,
        Parameters(KeyParam { key }): Parameters<KeyParam>,
    ) -> Result<CallToolResult, McpError> {
        if key > 0x0F {
            return Err(McpError::invalid_params("key must be 0x0-0xF", None));
        }
        self.send_cmd(json!({"cmd": "key_release", "key": key}))
            .await?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Key 0x{key:X} released"
        ))]))
    }

    #[tool(
        description = "Press and release a CHIP-8 hex key (0x0-0xF) after a short delay. Use this for single key taps."
    )]
    async fn key_press_and_release(
        &self,
        Parameters(KeyParam { key }): Parameters<KeyParam>,
    ) -> Result<CallToolResult, McpError> {
        if key > 0x0F {
            return Err(McpError::invalid_params("key must be 0x0-0xF", None));
        }
        self.send_cmd(json!({"cmd": "key_press", "key": key}))
            .await?;
        tokio::time::sleep(Duration::from_millis(200)).await;
        self.send_cmd(json!({"cmd": "key_release", "key": key}))
            .await?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Key 0x{key:X} pressed and released"
        ))]))
    }

    #[tool(
        description = "Press and release a CHIP-8 hex key (0x0-0xF) with a delay, then return the screen state. Use this to press a key and see the result."
    )]
    async fn key_tap_and_get_screen(
        &self,
        Parameters(KeyParam { key }): Parameters<KeyParam>,
    ) -> Result<CallToolResult, McpError> {
        if key > 0x0F {
            return Err(McpError::invalid_params("key must be 0x0-0xF", None));
        }
        self.send_cmd(json!({"cmd": "key_press", "key": key}))
            .await?;
        tokio::time::sleep(Duration::from_millis(200)).await;
        self.send_cmd(json!({"cmd": "key_release", "key": key}))
            .await?;

        let resp = self.send_cmd(json!({"cmd": "get_screen"})).await?;
        let pixels: Vec<Vec<bool>> =
            serde_json::from_value(resp["pixels"].clone()).unwrap_or_default();
        let screen = Chip8Debug::render_screen(&pixels);
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Key 0x{key:X} pressed and released\n\n```\n{screen}\n```"
        ))]))
    }

    #[tool(
        description = "Press and release a CHIP-8 hex key (0x0-0xF) with a delay, then return SCREEN DIFF. Shows only pixels that changed. Use this to see exactly what a key press does."
    )]
    async fn key_tap_and_get_diff(
        &self,
        Parameters(KeyParam { key }): Parameters<KeyParam>,
    ) -> Result<CallToolResult, McpError> {
        if key > 0x0F {
            return Err(McpError::invalid_params("key must be 0x0-0xF", None));
        }

        // 1. Screen before
        let before_resp = self.send_cmd(json!({"cmd": "get_screen"})).await?;
        let before: Vec<Vec<bool>> =
            serde_json::from_value(before_resp["pixels"].clone()).unwrap_or_default();

        // 2. Key press
        self.send_cmd(json!({"cmd": "key_press", "key": key})).await?;
        tokio::time::sleep(Duration::from_millis(200)).await;
        self.send_cmd(json!({"cmd": "key_release", "key": key})).await?;
        // Give emulator time to process key and settle at next FX0A
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 3. Screen after
        let after_resp = self.send_cmd(json!({"cmd": "get_screen"})).await?;
        let after: Vec<Vec<bool>> =
            serde_json::from_value(after_resp["pixels"].clone()).unwrap_or_default();

        // 4. Compute diff
        let (changes, mini_map, highlighted) = Self::diff_screens(&before, &after);

        let mut out = format!("Key 0x{key:X} pressed and released\n");

        if changes.is_empty() {
            out.push_str("Nessun pixel cambiato. Tasto non registrato o azione nulla.\n");
        } else {
            let on_count = changes.iter().filter(|c| c.2).count();
            let off_count = changes.len() - on_count;
            out.push_str(&format!("Pixel cambiati: {} ({} accesi, {} spenti)\n\n", changes.len(), on_count, off_count));
            out.push_str("Coordinate:\n");
            for (x, y, on) in &changes {
                out.push_str(&format!("  ({x:2},{y:2}) -> {}\n", if *on { "ON " } else { "OFF" }));
            }
            out.push('\n');
            out.push_str(&mini_map);
            out.push_str("\n\nSchermo con evidenziazioni (@=nuovo ·=spento █=invariato):\n");
            out.push_str(&format!("```\n{highlighted}\n```"));
        }

        Ok(CallToolResult::success(vec![Content::text(out)]))
    }
}

// ----- server handler -----

#[tool_handler]
impl ServerHandler for Chip8Debug {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_server_info(Implementation::from_build_env())
    }
}

// ----- entry point -----

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "chip8_mcp=info".into()),
        )
        .init();

    let port: u16 = std::env::var("CHIP8_DEBUG_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(9876);

    let server = Chip8Debug { port };
    server.serve(stdio()).await?.waiting().await?;

    Ok(())
}
