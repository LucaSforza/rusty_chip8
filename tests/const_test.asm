; Test .const values used as operands
.const SCREEN_W = 64
.const SPRITE_H = 5
.const CELL_W = 8

    LD V0, #$00
    ADD V0, CELL_W
    SE V0, SCREEN_W
    LD V1, SPRITE_H
    RND V2, SPRITE_H
    DRW V0, V1, SPRITE_H
