# TODO

Priority-ordered list of known issues and improvements.

## High

- [ ] **`to_draw` never reset to `false`** — `interpreter.rs:302` sets `to_draw = true` but nothing clears it. After first sprite draw, every frame redraws the full 640x320 buffer. CPU waste.
- [ ] **Timer thread doesn't decrement correctly** — `registers.rs:34` uses `(difference.as_millis() % 16) as u8` instead of decrementing by 1 at 60Hz. Delay/sound timers don't reach 0 after exactly N/60 seconds. Breaks game timing (e.g. chipwar.ch8).
- [ ] **No `Result` types, `exit(1)` everywhere** — ~8 `exit(1)` calls across interpreter, display, memory, registers. Any runtime error kills the process. Untestable, undebuggable.
- [ ] **Keyboard lock contention** — `DataKeys` shared via `Arc<Mutex>`. Multiple fields locked independently. Anti-pattern: should design ownership properly instead of mutex-as-toy.

## Medium

- [ ] **`sub_regs`/`subn_regs` manual overflow** — `interpreter.rs:232` does `x_value as u16 + 0b1_0000_0000` instead of `wrapping_sub`. Works but fragile.
- [ ] **Draw uses `!pixel_color`** — `display.rs:58` inverts u32 instead of using an explicit color constant. Works only because pixel is always 0 or 0xFFFFFFFF.
- [ ] **Memory capacity check uses `.capacity()` not `.len()`** — `memory.rs:47`. Works by accident (Vec allocated exactly to CAPACITY) but semantically wrong.
- [ ] **Typo: `Istruction`** — pervasive in `interpreter.rs`, should be `Instruction`.

## Low

- [ ] **`write_rom_on_mem` reads file byte-by-byte** — `interpreter.rs:113` loops `for byte in file.bytes()`. Should use `fs::read()`.
- [ ] **Keyboard field `i: Mutex<usize>` never read** — `keyboard.rs:15`. Incremented in `remove()` but never used. Dead code.
- [ ] **Font address 0x50 hardcoded in two places** — `memory.rs:32` and `interpreter.rs:339`. Should be a shared constant.
- [ ] **`ONEHERTZ` misnamed** — `keyboard.rs:8` declares `ONEHERTZ = 1.0/60.0` but the value is ~16.6ms, not 1Hz. Should be `TICK_INTERVAL` or similar.
