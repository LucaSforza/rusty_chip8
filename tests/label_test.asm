; Test labels, jumps, calls
    LD V0, #$05
    LD V1, #$03
    ADD V0, V1
    SE V0, #$08
    JP skip
    LD V2, #$FF
skip:
    CALL sub
    JP end
sub:
    LD V3, #$AA
    RET
end:
    .byte 0x00
