; CHIP-8 IBM logo test using system font — loops forever
    LD V0, #$00        ; x = 0
    LD V1, #$00        ; y = 0
    LD V2, #$08        ; spacing = 8

loop:
    LD V3, V0          ; save x (unused, tests LdVV)
    LD F, V0           ; I = font sprite for digit V0
    DRW V0, V1, #$05   ; draw 5-byte sprite at (x, y)
    ADD V0, V2         ; advance x
    SE V0, #$40        ; drawn 8 chars? (64/8 = 8)
    JP loop

    ; Loop forever once done
endless:
    JP endless
