# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commit rules

- Write all commit messages in English
- Use conventional commits style (feat:, fix:, refactor:, chore:, docs:)

## Build & Run

```bash
cargo build                         # debug build (workspace)
cargo run <path_to_rom>             # run emulator
cargo run <path> -- --speed 10 --fps 60  # with options
cargo run -p chip8-mcp              # MCP debug server (connects to emulator)
cargo run -p chip8-asm -- input.asm -o rom.ch8  # assemble ROM
```

Args (emulator): `--speed/-s <cycles per frame>` (default 100), `--fps/-f <target fps>` (default 60),
`--debug` (default true, TCP debug server), `--debug-port` (default 9876).

Args (assembler): `input.asm`, `-o <output.ch8>` (default `a.out.ch8`), `-l <listing.txt>`.

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
`key_tap_and_get_screen(key, path?)`, `key_tap_and_get_diff`, `screen_script`.

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
Player sprite (MAN) = `0x00 0x60 0x60 0x00` = 2x2 block at sprite offset (1,1).

**Controls** (overworld):
- `0x5` → W = north, `0x7` → A = west, `0x9` → D = east, `0x8` → S = south
- At board edges, direction wraps and may change board.
- EDGE direction shown by detect_walls.py as board transition.

**Special tiles** (from game source at ROM 0xB01-0xB21):
- Events: 16 (x,y) coords. Stepping on tile triggers scripted scene.
- Secret wall: board 0x6 column 7 — rows 2-7 passable after rumble event.
- Gates: board 0x4 columns 8-12 — path is always walkable, gate sprite is cosmetic.

**Block puzzle mode**: Entered via event tiles. A/D=move, E=pick up/drop, Q=reset.

**Player detection**: Vc=V12=manx, Vb=V11=manx, Vd=V13=boardno.
- `screen_script(path="scripts_games/caveexplorer/detect_walls.py")` — walls + special notes
- `screen_script(path="scripts_games/caveexplorer/board_map.py")` — full 16x8 tile map

**Play via MCP**: Use `key_tap_and_get_screen(key, repeat=N, path=...)` for moves. `repeat=5` presses same key N times in one call — prefer over calling tool repeatedly. `path="scripts/foo.py"` runs script on resulting state. No path = full state JSON. `key_tap_and_get_diff(key)` for pixel diff of a single press.

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
mcp-server/          # chip8-mcp crate — MCP stdio server
  └── src/main.rs    # debug tools (get_screen, get_registers, step, breakpoints, etc.)
chip8-asm/           # assembler binary crate
  └── src/
      ├── main.rs    # CLI, two-pass assembly
      ├── lexer.rs   # tokenizer
      ├── parser.rs  # instruction/directive parser
      ├── encoder.rs # all 35 opcodes → [u8; 2]
      └── symbol.rs  # label + constant table
.mcp.json  # MCP server config for Claude Code
```

MCP server communicates with emulator via TCP localhost (JSON lines protocol, port 9876 by default).

### Assembler language

Syntax: `;`/`#` comments, `label:` labels, `V0`-`VF`/`I`/`DT`/`ST` registers, `#$FF`/`$FF`/`0xFF` immediates.
Directives: `.org`, `.byte`, `.word`, `.ascii`, `.asciz`, `.align`, `.space`, `.const`.
All 35 standard instructions. Two-pass assembly for forward label references.
`.const` values usable as immediate operands (e.g., `LD V0, MY_CONST`).
