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
cargo run -p chip8-lsp              # LSP server (stdio mode)
```

### Justfile

```bash
just build-lsp              # Build chip8-lsp
just test-lsp               # Run LSP tests (2 unit + 1 integration)
just build-lsp-release      # Release build
just run-lsp                # Run LSP in stdio mode
just install-lsp-nvim       # Build + copy binary + print Neovim config
just install-lsp-vscode     # Build + package VSIX extension
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

### LSP Server

Language server for CHIP-8 assembly (`chip8-lsp`). Registers in `.asm`/`.chip8`/`.ch8` files.

```bash
# Run directly:
cargo run -p chip8-lsp

# Configure in opencode.jsonc:
# "lsp": { "chip8": { "command": ["cargo", "run", "-p", "chip8-lsp"], "extensions": [".asm", ".chip8"] } }

# Neovim: just install-lsp-nvim
# VSCode:  just install-lsp-vscode
```

Provides: diagnostics, hover, go-to-definition, find-references, completions,
document/workspace symbols, semantic highlighting, rename.

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
- FPS display printed to stdout once per second
- Debugger TCP thread accepts one connection at a time, processes JSON commands, returns JSON responses
- LSP server uses tower-lsp 0.20 + lsp-types 0.94; all handlers (hover, definition, completion, symbols, semantic tokens, rename)
- Partial analysis: even with parse errors, tokens and symbol table are preserved so hover/completion work on valid portions
- LSP integration test sends real LSP protocol messages over stdio; 2 unit tests + 1 integration test

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

### pong (examples/pong/)

Pong game written in assembler demonstrating all new features (includes, macros, structures, local labels).

**Building**: `cargo run -p chip8-asm -- examples/pong/pong.asm -o pong.ch8`

**Controls**:
- Menu: press `1` (0x1) for 1-player, `2` (0x2) for 2-player
- P1 up: `W` (0x5), P1 down: `S` (0x8)
- P2 up: `E` (0x6), P2 down: `D` (0x9)
- First to 5 wins, then press any key to restart

**AI** (1P mode): Right paddle tracks ball Y position with a 3px center offset.
**Structure**: `include "const.asm"` → `.const` values + `struct GameState` → main game code.
Uses built-in CHIP-8 font for score (0/1/2/3/4/5) and menu digits.

## Workspace

```text
rusty_chip8/         # emulator binary (crate root)
mcp-server/          # chip8-mcp crate — MCP stdio server
  └── src/main.rs    # debug tools (get_screen, get_registers, step, breakpoints, etc.)
chip8-asm/           # assembler crate (lib + binary)
  └── src/
      ├── lib.rs         # Public API, assemble(), pipeline driver
      ├── main.rs        # Thin CLI wrapper
      ├── lexer.rs       # Tokenizer (LBrace/RBrace, dotted identifiers)
      ├── parser.rs      # Instruction/directive/struct parser
      ├── encoder.rs     # All 35 opcodes → [u8; 2] — UNCHANGED
      ├── symbol.rs      # Label + constant table
      ├── sourcemap.rs   # SourceMap for cross-file diagnostics
      ├── include.rs     # IncludeResolver + FileProvider + cycle detection
      ├── macroexpand.rs # Macro collector + expander + local labels
      └── preprocess.rs  # Orchestrates include → macro pipeline
  └── tests/
      └── integration.rs # 12 assembler tests
chip8-lsp/           # LSP server crate
  └── src/
      ├── main.rs        # Entry point
      ├── server.rs      # tower-lsp Backend (all handlers)
      ├── document.rs    # Per-document analysis state
      ├── workspace.rs   # File discovery + include graph
      ├── diagnostics.rs # Error mapping LSP
      ├── hover.rs       # Hover info (instrs, regs, dirs, symbols)
      ├── definition.rs  # Go to definition
      ├── references.rs  # Find references (cross-workspace)
      ├── completion.rs  # Context-aware completions
      ├── symbols.rs     # Document/workspace symbols
      ├── highlight.rs   # Semantic tokens
      └── rename.rs      # Safe rename for labels/consts/macros
  └── tests/
      └── lsp_test.rs    # Integration test (LSP protocol via stdio)
vscode-chip8/        # VS Code extension (thin client)
  ├── package.json
  ├── src/extension.js
  ├── language-configuration.json
  └── syntaxes/chip8.tmGrammar.json
examples/
  └── pong/              # Pong game demonstrating new assembler features
      ├── pong.asm       # Main game (include, .const values)
      └── const.asm      # Constants + struct GameState
justfile  # Build/test/install commands
.mcp.json  # MCP server config for Claude Code
```

MCP server communicates with emulator via TCP localhost (JSON lines protocol, port 9876 by default).

### Assembler language

Syntax: `;`/`#` comments, `label:` labels, `V0`-`VF`/`I`/`DT`/`ST` registers, `#$FF`/`$FF`/`0xFF` immediates.
Directives: `.org`, `.byte`, `.word`, `.ascii`, `.asciz`, `.align`, `.space`, `.const`.
All 35 standard instructions. Two-pass assembly for forward label references.
`.const` values usable as immediate operands (e.g., `LD V0, MY_CONST`).
`.byte` and `.word` accept symbolic references (e.g., `.word Sprite.width`).

#### Preprocessing pipeline

Source files flow through a preprocessing pipeline before lexing/parsing:

```
Source Files → IncludeResolver → MacroProcessor → MacroExpander → Lexer → Parser → compute_layout → generate_code → ROM
```

#### Include system

```asm
include "constants.asm"
include "sprites/player.asm"
```

- Processed before lexing — textual include resolution.
- Relative paths resolved from the including file.
- Nested includes supported; include cycles detected with clear error.
- Duplicate includes allowed (FASM-style).
- Source locations preserved for diagnostics via `SourceMap`.

#### Macro system

```asm
macro clear_screen {
    CLS
}

macro load_const reg, value {
    LD reg, value
}

clear_screen
load_const V0, 10
```

- Text-level expansion before lexing — independent from instruction parsing.
- Parameterized macros; zero-argument macros; multi-line bodies.
- Single-line bodies: `macro name { body }` or `macro name arg { body }`
- Recursive expansion detection with clear diagnostics.
- Wrong argument count produces error at invocation site.
- Macros shadow CHIP-8 instructions by name; arg-count mismatch → falls through to instruction parsing.

#### Local labels inside macros

```asm
macro spin reg {
%%loop:
    ADD reg, 1
    JP %%loop
}
```

- `%%name:` defines a local label; `%%name` references it.
- Each invocation generates unique labels: `__m1_loop`, `__m2_loop`, etc.
- Counter is global across all macro invocations — no collisions with user labels.
- Works with forward references via standard two-pass resolution.

#### Structures

```asm
struct Sprite {
    x byte
    y byte
    width byte
    height byte
}
```

- Generates constants: `Sprite.x = 0`, `Sprite.y = 1`, `Sprite.width = 2`, `Sprite.height = 3`, `Sprite.SIZE = 4`.
- Supports `byte` and `word` fields; word fields advance offset by 2.
- Structure definition is compile-time metadata only — no runtime representation.
- Integrated with existing symbol table; `.const` entries created during `compute_layout`.
- Dotted identifiers (e.g., `Sprite.width`) are tokenized as single `Word` tokens in lexer.

#### Assembler module layout

```
chip8-asm/src/
├── lib.rs              # Public API, AssemblyOptions, assemble(), assemble_file(),
│                       # analyze(), analyze_with(), compute_layout, generate_code
├── main.rs             # Thin CLI wrapper
├── lexer.rs            # Tokenizer (LBrace/RBrace, dotted identifiers)
├── parser.rs           # Parser (struct keyword, symbolic .byte/.word)
├── encoder.rs          # Opcode encoder — UNCHANGED
├── symbol.rs           # Symbol table + labels()/constants() iterators
├── sourcemap.rs        # SourceMap: Vec<(file, line)> + resolve_pos()
├── include.rs          # IncludeResolver + FileProvider trait + cycle detection
├── macroexpand.rs      # Phase 1: collect_definitions, Phase 2: expand + local labels
└── preprocess.rs       # Orchestrates include → macro collect → macro expand

chip8-asm/tests/
└── integration.rs      # 12 tests covering includes, macros, local labels, structs, recursion
```

#### Pong game example

`examples/pong/` — demonstrates all new features:

```bash
cargo run -p chip8-asm -- examples/pong/pong.asm -o pong.ch8
cargo run -- pong.ch8
```

- `pong.asm` — main game with `include "const.asm"`
- `const.asm` — `.const` values + `struct GameState` for documentation
- 1-player (vs AI) or 2-player mode, W/S and E/D controls, first to 5 wins
- Menu: press 1 for 1P, 2 for 2P
- Key mapping: P1 up=W(0x5), P1 down=S(0x8), P2 up=E(0x6), P2 down=D(0x9)
