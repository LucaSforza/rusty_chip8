# rusty_chip8

CHIP-8 emulator + MCP debug server + assembler in Rust.

## Crates

| Crate | Purpose |
|-------|---------|
| `rusty_chip8` | Emulator core + minifb window + debug TCP server |
| `chip8-mcp` | MCP stdio server, exposes debug tools to Claude |
| `chip8-asm` | MIPS-like assembler, input `.asm` → output `.ch8` |

## Emulator Usage

```bash
cargo run <rom.ch8>                    # run with defaults
cargo run <rom.ch8> -- --speed 500     # 500 insts/frame
cargo run <rom.ch8> -- --fps 30       # cap at 30 FPS
```

Keyboard: `X`=0, `1234`=1-3, `QWEASDZC4RFV`=4-F.

## MCP Debug Server

Built-in TCP server on port 9876. Start emulator, then:

```bash
cargo run -p chip8-mcp
```

Or via `.mcp.json` for Claude Code auto-spawn. Tools: `get_screen`, `get_registers`, `get_memory`, `step`, `pause`/`resume`, `set_breakpoint`/`clear_breakpoint`, `get_state`, `key_press`/`key_release`, `key_tap_and_get_screen`, `key_tap_and_get_diff`, `screen_script`.

## Assembler

```bash
cargo run -p chip8-asm -- input.asm -o rom.ch8 -l listing.txt
```

Language — MIPS-like:

```asm
; comment
.const FOO = 42
start:
    LD V0, #$0F       ; immediate (also $0F, 0x0F, FOO)
    LD V1, V2         ; register copy
    LD V0, DT         ; delay timer
    LD V0, K          ; wait key
    LD DT, V0
    LD ST, V0
    LD F, V0          ; font sprite
    LD B, V0          ; BCD store
    LD [I], V5        ; store V0-V5 at I
    LD V5, [I]        ; load V0-V5 from I
    ADD V0, #$0A
    ADD I, V0
    SUB V0, V1
    SE V0, #$0F       ; skip if equal byte
    SE V0, V1         ; skip if equal reg
    SNE V0, #$0F
    JP label
    CALL label
    RET
    DRW V0, V1, #5
    RND V0, #$FF
    CLS
    SHR V0
    SHL V0
    .byte 0x12, 0x34
    .word 0xFACE
    .ascii "hello"
    .asciz "world"
    .align 2
    .space 16
```

Registers: `V0`-`VF` (8-bit), `I` (16-bit addr), `DT`, `ST`.
Labels end with `:`. `.const NAME = value` for symbolic constants.
All 35 standard CHIP-8 instructions supported.

## Resources

- [Cowgod's Chip-8 Technical Reference](http://devernay.free.fr/hacks/chip8/C8TECH10.HTM)
- [Octo examples](https://github.com/JohnEarnest/Octo/tree/gh-pages/examples)
- [Glitch Ghost](https://github.com/jackiekircher/glitch-ghost)

## TODO

- Bug in chipwar.ch8: conquering a region often ends it prematurely
