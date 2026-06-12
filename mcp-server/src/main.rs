use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::transport::stdio;
use rmcp::{ErrorData as McpError, ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
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
