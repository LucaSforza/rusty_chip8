# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo build                    # debug build
cargo build --release          # release build
cargo run <path_to_rom>        # run emulator
cargo run <path> -- --speed 10 --fps 60  # with options
```

Args: `--speed/-s <cycles per frame>` (default 100), `--fps/-f <target fps>` (default 60).

### Test ROMs

Located in `tests/`. Run with: `cargo run tests/<rom>`.

- `1-chip8-logo.ch8`, `2-ibm-logo.ch8` — basic display tests
- `3-corax+.ch8`, `4-flags.ch8`, `5-quirks.ch8` — instruction test suites
- `6-keypad.ch8` — keyboard test
- `7-beep.ch8` — sound test

## Architecture

### Chip-8 spec
- 64x32 pixel display, 4KB memory, 16 general-purpose V-registers (8-bit), I-register (16-bit)
- Stack for subroutine return addresses, delay/sound timers decremented at 60Hz
- Programs loaded at 0x200, font data at 0x50

### Source files (`src/`)

- **`main.rs`** — Entry point. Config struct parses CLI args. minifb window loop: runs N instructions per frame (`--speed`), handles sound via rodio (700Hz sine wave), calls draw when display dirty.
- **`interpreter.rs`** — Core `Interpreter` struct. Parse 16-bit opcode into `Istruction` (opcode, reg, nibbles, func_code, addr, byte). Fetch-decode-execute loop in `next_istr()`. All CHIP-8 opcodes (0x0–0xF) decoded via match on opcode + func_code/byte.
- **`display.rs`** — `Display` struct with 64x32 bool buffer. `add_sprite()` XOR-sprites onto display, returns collision flag. `draw()` maps buffer to 10x upscaled minifb pixel buffer (640x320).
- **`keyboard.rs`** — `DataKeys` (thread-safe key buffer behind Arc<Mutex>) and `KeyboardState` (minifb `InputCallback` impl). Key mapping: X=0x0, 1-4=0x1-0x3, QWEASDZC4RFV for 0x4-0xF. `wait_key_pressed` blocks execution until new key press.
- **`memory.rs`** — `Memory` struct wraps 4096-byte Vec. Reads/writes slices and 16-bit big-endian words. Font sprites (0x0-0xF) loaded at 0x50 on init.
- **`registers.rs`** — `Registers` struct with V[0..15], I, PC, stack. Spawns background thread that decrements delay/sound timers at ~60Hz using `ONEHERTZ` (1/60s) sleep intervals.

### Key design notes
- Timers decrement in a background thread using `thread::sleep(Duration::from_secs_f64(1.0/60.0))` — approximate 60Hz tick
- Keyboard uses `minifb::InputCallback` trait, state stored behind `Arc<Mutex>` for shared access between main thread and minifb callback
- No test framework currently — validation is manual via test ROMs
- FPS display printed to stdout once per second
