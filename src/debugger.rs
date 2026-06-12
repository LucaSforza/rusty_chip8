use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use serde::Serialize;

use crate::display::Display;
use crate::memory::Memory;
use crate::registers::Registers;

#[derive(Clone, Serialize)]
pub struct SharedState {
    pub pixels: Vec<Vec<bool>>,
    pub v_regs: [u8; 16],
    pub i: u16,
    pub pc: u16,
    pub stack: Vec<u16>,
    pub delay: u8,
    pub sound: u8,
    pub memory: Vec<u8>,
}

pub struct Debugger {
    pub state: Arc<Mutex<SharedState>>,
    pub breakpoints: Arc<Mutex<HashSet<u16>>>,
    pub paused: Arc<AtomicBool>,
    pub step_requested: Arc<AtomicBool>,
    pub running: Arc<AtomicBool>,
}

impl Debugger {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(SharedState {
                pixels: vec![vec![false; 64]; 32],
                v_regs: [0; 16],
                i: 0,
                pc: 0,
                stack: Vec::new(),
                delay: 0,
                sound: 0,
                memory: vec![0; 4096],
            })),
            breakpoints: Arc::new(Mutex::new(HashSet::new())),
            paused: Arc::new(AtomicBool::new(false)),
            step_requested: Arc::new(AtomicBool::new(false)),
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn update_state(&self, display: &Display, regs: &Registers, memory: &Memory) {
        let mut state = self.state.lock().unwrap();
        state.pixels = display.buf();
        state.v_regs = regs.all_v();
        state.i = regs.get_i();
        state.pc = regs.get_pc();
        state.stack = regs.stack_snapshot();
        state.delay = regs.get_delay();
        state.sound = regs.get_sound();
        state.memory.copy_from_slice(memory.as_slice());
    }

    pub fn spawn_listener(self: &Arc<Self>, port: u16) {
        let this = self.clone();
        thread::spawn(move || {
            let addr = format!("127.0.0.1:{port}");
            let listener = match TcpListener::bind(&addr) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("[DEBUGGER] failed to bind {addr}: {e}");
                    return;
                }
            };
            eprintln!("[DEBUGGER] listening on {addr}");

            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        eprintln!("[DEBUGGER] client connected");
                        if let Err(e) = this.handle_client(stream) {
                            eprintln!("[DEBUGGER] client error: {e}");
                        }
                        eprintln!("[DEBUGGER] client disconnected");
                    }
                    Err(e) => {
                        eprintln!("[DEBUGGER] accept error: {e}");
                    }
                }
                if !this.running.load(Ordering::Relaxed) {
                    break;
                }
            }
        });
    }

    fn handle_client(&self, stream: TcpStream) -> std::io::Result<()> {
        let mut reader = BufReader::new(&stream);
        let mut writer = &stream;
        let mut line = String::new();

        loop {
            line.clear();
            let n = reader.read_line(&mut line)?;
            if n == 0 {
                break; // connection closed
            }

            let req: serde_json::Value = match serde_json::from_str(line.trim()) {
                Ok(v) => v,
                Err(e) => {
                    let err = serde_json::json!({"error": format!("invalid JSON: {e}")});
                    writeln!(writer, "{err}")?;
                    writer.flush()?;
                    continue;
                }
            };

            let cmd = req.get("cmd").and_then(|c| c.as_str()).unwrap_or("");
            let resp = self.handle_command(cmd, &req);
            let json = serde_json::to_string(&resp)?;
            writeln!(writer, "{json}")?;
            writer.flush()?;

            if !self.running.load(Ordering::Relaxed) {
                break;
            }
        }
        Ok(())
    }

    fn handle_command(&self, cmd: &str, req: &serde_json::Value) -> serde_json::Value {
        match cmd {
            "get_screen" | "get_registers" => {
                let state = self.state.lock().unwrap();
                serde_json::to_value(&*state).unwrap_or_default()
            }
            "get_memory" => {
                let start = req.get("s").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let end = req
                    .get("e")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(4096) as usize;
                let state = self.state.lock().unwrap();
                let data: Vec<u8> = if end > state.memory.len() {
                    state.memory[start..].to_vec()
                } else {
                    state.memory[start..end].to_vec()
                };
                serde_json::json!({"data": data})
            }
            "set_bp" => {
                let addr = req.get("a").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
                self.breakpoints.lock().unwrap().insert(addr);
                serde_json::json!({"ok": true})
            }
            "clear_bp" => {
                let addr = req.get("a").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
                self.breakpoints.lock().unwrap().remove(&addr);
                serde_json::json!({"ok": true})
            }
            "step" => {
                self.step_requested.store(true, Ordering::Relaxed);
                self.paused.store(true, Ordering::Relaxed);
                serde_json::json!({"ok": true})
            }
            "pause" => {
                self.paused.store(true, Ordering::Relaxed);
                serde_json::json!({"ok": true})
            }
            "continue" => {
                self.paused.store(false, Ordering::Relaxed);
                serde_json::json!({"ok": true})
            }
            "stop" => {
                self.running.store(false, Ordering::Relaxed);
                serde_json::json!({"ok": true})
            }
            "get_state" => {
                let state = self.state.lock().unwrap();
                let mut mem_first = vec![0u8; 256];
                mem_first.copy_from_slice(&state.memory[..256]);
                let resp = serde_json::json!({
                    "pixels": state.pixels,
                    "v": state.v_regs,
                    "i": state.i,
                    "pc": state.pc,
                    "stack": state.stack,
                    "dt": state.delay,
                    "st": state.sound,
                    "memory": mem_first,
                });
                resp
            }
            _ => serde_json::json!({"error": format!("unknown cmd: {cmd}")}),
        }
    }
}
