# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commit rules

- Write all commit messages in English
- Use conventional commits style (feat:, fix:, refactor:, chore:, docs:)

## Build & Run

```bash
cargo build                    # debug build (workspace)
cargo build --release          # release build
cargo run <path_to_rom>        # run emulator
cargo run <path> -- --speed 10 --fps 60  # with options
cargo run -p chip8-mcp         # run MCP debug server (connects to emulator)
```

Args: `--speed/-s <cycles per frame>` (default 100), `--fps/-f <target fps>` (default 60),
`--debug` (default true, TCP debug server), `--debug-port` (default 9876).

### Test ROMs

Located in `tests/`. Run with: `cargo run tests/<rom>`.

- `1-chip8-logo.ch8`, `2-ibm-logo.ch8` — basic display tests
- `3-corax+.ch8`, `4-flags.ch8`, `5-quirks.ch8` — instruction test suites
- `6-keypad.ch8` — keyboard test
- `7-beep.ch8` — sound test

### Debug MCP Server

Emulator includes TCP debug server (enabled by default on port 9876).
MCP server crate (`chip8-mcp`) connects to it and exposes tools for Claude.

```bash
# Terminal 1: emulator
cargo run -- tests/2-ibm-logo.ch8

# Terminal 2: MCP server (or via .mcp.json auto-spawn)
cargo run -p chip8-mcp
```

Tools: `get_screen`, `get_registers`, `get_memory`, `set_breakpoint`,
`clear_breakpoint`, `step`, `pause`, `continue`, `stop`, `get_state`,
`key_press`, `key_release`, `key_press_and_release`,
`screen_script`.

## Architecture

### Chip-8 spec
- 64x32 pixel display, 4KB memory, 16 general-purpose V-registers (8-bit), I-register (16-bit)
- Stack for subroutine return addresses, delay/sound timers decremented at 60Hz
- Programs loaded at 0x200, font data at 0x50

### Source files (`src/`)

- **`main.rs`** — Entry point. Clap CLI args. minifb window loop: runs N instructions per frame (`--speed`), handles sound via rodio (700Hz sine wave), calls draw when display dirty. Debug server integration.
- **`interpreter.rs`** — Core `Interpreter` struct. Parse 16-bit opcode into `Istruction`. Fetch-decode-execute loop in `next_istr()`. Integrated with debugger for pause/step/breakpoints.
- **`display.rs`** — `Display` struct with 64x32 bool buffer. `add_sprite()` XOR-sprites onto display, returns collision flag. `draw()` maps buffer to 10x upscaled minifb pixel buffer (640x320).
- **`keyboard.rs`** — `DataKeys` (thread-safe key buffer behind Arc<Mutex>) and `KeyboardState` (minifb `InputCallback` impl). Key mapping: X=0x0, 1-4=0x1-0x3, QWEASDZC4RFV for 0x4-0xF.
- **`memory.rs`** — `Memory` struct wraps 4096-byte Vec. Reads/writes slices and 16-bit big-endian words. Font sprites loaded at 0x50 on init.
- **`registers.rs`** — `Registers` struct with V[0..15], I, PC, stack. Background thread decrements delay/sound timers at ~60Hz.
- **`debugger.rs`** — TCP debug server thread on localhost. `SharedState` (snapshot of display+regs+memory behind Arc<Mutex>). Control flags for pause/step/breakpoints/running (AtomicBool). JSON protocol over newline-delimited TCP.

### Key design notes
- Timers decrement in a background thread using `thread::sleep(Duration::from_secs_f64(1.0/60.0))` — approximate 60Hz tick
- Keyboard uses `minifb::InputCallback` trait, state stored behind `Arc<Mutex>` for shared access between main thread and minifb callback
- No test framework currently — validation is manual via test ROMs
- FPS display printed to stdout once per second
- Debugger TCP thread accepts one connection at a time, processes JSON commands, returns JSON responses

## Game ROMs

Example ROMs in `examples/`. Run with `--speed 1000` for responsive gameplay (timer fix needed).

### caveexplorer.ch8

Cave exploration / block-pushing puzzle game by David S. Moore.
Overworld with 16 boards (0x0-0xF), each 16x8 tiles (4x4 pixels).
Player sprite (MAN) is `0x00 0x60 0x60 0x00` = 2x2 pixel block at bitmap offset.

**Controls** (overworld):
- `0x5` → W = north (many-=1)
- `0x7` → A = west  (manx-=1)
- `0x9` → D = east  (manx+=1)
- `0x8` → S = south (many+=1)

**Board transitions**: At map edges, press direction to wrap & change board.
Transition tables: board-n[board]=board, board-e[b], board-s[b], board-w[b].

**Block puzzle mode**: Entered via special tiles. A/D move, E pick up/drop block, Q reset.

**Player detection** (registers):
- `Vc(V12)` = manx (tile X 0-15), `Vb(V11)` = many (tile Y 0-7), `Vd(V13)` = boardno
- Script: `screen_script(path="scripts_games/caveexplorer/detect_walls.py")`

**Board data**: 16 bytes per board, one per column. Bit=1 = path (walkable).

**Play via MCP**: Single `key_press_and_release(key)` per move. Use `key_tap_and_get_diff` for pixel diff. Use `screen_script` for analysis.

### fez.ch8

World-rotation puzzle (FEZ clone). Keys rotate perspective:
- `0x7` (A) / `0x4` (Q) = rotate left
- `0x9` (D) / `0x6` (E) = rotate right
- `0x5` (W) = up/rotate
- `0x8` (S) = down/rotate

### slippery.ch8

Puzzle/platformer. Controls TBD.

## Workspace

```text
rusty_chip8/         # emulator binary (crate root)
mcp-server/          # chip8-mcp crate — MCP stdio server using rmcp
  └── src/main.rs    # tools: get_screen, get_registers, get_memory, breakpoints, step, pause, continue, stop, get_state, key_press, key_release, key_press_and_release
.mcp.json  # MCP server config for Claude Code (spawns `cargo run -p chip8-mcp`)
```

MCP server communicates with emulator via TCP localhost (JSON lines protocol, port 9876 by default).
