; Test file for CHIP-8 LSP
.const SCREEN_W = 64
.const SCREEN_H = 32

start:
    CLS
    LD V0, SCREEN_W
    LD V1, 10
    ADD V0, V1
    DRW V0, V1, 5
    JP start

; This should produce an error:
    INVALID_OPCODE V0, V1
