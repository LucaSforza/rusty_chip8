; Directive tests
.org 0x300
const_val:
    .const FOO = 42
    .byte 0x12, 0x34, 0xAB
    .word 0xFACE, 0xBEEF
    .ascii "hello"
    .asciz "world"
    .align 2
    .space 4
    .byte 0xFF
