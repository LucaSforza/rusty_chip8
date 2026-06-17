; Pong — CHIP-8 game
; 1-player (vs AI) or 2-player mode
; Controls: W/S for P1, E/D for P2, AI in 1P mode

include "const.asm"

.org 0x200

; ── Sprite data ─────────────────────────────────────────────────────────

paddle_sprite:
.byte 0xC0, 0xC0, 0xC0, 0xC0, 0xC0, 0xC0

ball_sprite:
.byte 0x80

; ── Init ────────────────────────────────────────────────────────────────

main:
    LD V0, 13
    LD V1, 13
    LD V2, BALL_START_X
    LD V3, BALL_START_Y
    LD V4, 0
    LD V5, 0
    LD V7, 1
    LD V8, 1
    JP menu

; ── Menu: press 1 for 1P, 2 for 2P ──────────────────────────────────────

menu:
    CLS
    LD V9, 1
    LD F, V9
    LD V9, 26
    LD VA, 13
    DRW V9, VA, 5
    LD V9, 2
    LD F, V9
    LD V9, 38
    DRW V9, VA, 5
    LD V9, K
    SE V9, KEY_1
    JP menu_chk2
    LD V6, MODE_1P
    JP game_loop
menu_chk2:
    SE V9, KEY_2
    JP menu
    LD V6, MODE_2P

; ── Main game loop ──────────────────────────────────────────────────────

game_loop:
    CLS

    ; ── Draw P1 score ──
    LD V9, V4
    LD F, V9
    LD V9, SCORE_X1
    LD VA, SCORE_Y
    DRW V9, VA, 5

    ; ── Draw P2 score ──
    LD V9, V5
    LD F, V9
    LD V9, SCORE_X2
    LD VA, SCORE_Y
    DRW V9, VA, 5

    ; ── Draw P1 paddle ──
    LD I, paddle_sprite
    LD V9, PADDLE_X1
    LD VA, V0
    DRW V9, VA, PADDLE_H

    ; ── Draw P2 paddle ──
    LD I, paddle_sprite
    LD V9, PADDLE_X2
    LD VA, V1
    DRW V9, VA, PADDLE_H

    ; ── P1 input ──
    LD VE, KEY_P1_UP
    SKP VE
    JP p1_no_up
    SE V0, 0
    JP p1_up_go
    JP p1_no_up
p1_up_go:
    LD V9, 1
    SUB V0, V9
p1_no_up:

    LD VE, KEY_P1_DN
    SKP VE
    JP p1_no_dn
    LD V9, V0
    SE V9, MAX_PADDLE_Y
    JP p1_dn_go
    JP p1_no_dn
p1_dn_go:
    ADD V0, 1
p1_no_dn:

    ; ── P2 input / AI ──
    SE V6, MODE_2P
    JP ai_control

    LD VE, KEY_P2_UP
    SKP VE
    JP p2_no_up
    SE V1, 0
    JP p2_up_go
    JP p2_no_up
p2_up_go:
    LD V9, 1
    SUB V1, V9
p2_no_up:

    LD VE, KEY_P2_DN
    SKP VE
    JP p2_input_done
    LD V9, V1
    SE V9, MAX_PADDLE_Y
    JP p2_dn_go
    JP p2_input_done
p2_dn_go:
    ADD V1, 1
p2_input_done:
    JP after_input

    ; ── AI control (1P mode) ──
ai_control:
    LD V9, V1
    ADD V9, 3
    LD VA, V3
    SUBN V9, VA
    SE VF, 1
    JP ai_up

    SE V1, MAX_PADDLE_Y
    JP ai_dn_go
    JP after_input
ai_dn_go:
    ADD V1, 1
    JP after_input

ai_up:
    SE V1, 0
    JP ai_up_go
    JP after_input
ai_up_go:
    LD V9, 1
    SUB V1, V9

after_input:

    ; ── Draw ball ──
    LD I, ball_sprite
    DRW V2, V3, 1

    ; ── Move ball ──
    SE V7, 0
    JP ball_right
    SE V2, 0
    JP ball_l_ok
    JP p2_scores
ball_l_ok:
    LD V9, 1
    SUB V2, V9
    JP ball_y

ball_right:
    ADD V2, 1
    JP ball_y

ball_y:
    SE V8, 0
    JP ball_down
    SE V3, 0
    JP ball_u_ok
    LD V8, 1
    JP ball_done_move
ball_u_ok:
    LD V9, 1
    SUB V3, V9
    JP ball_done_move

ball_down:
    SE V3, 31
    JP ball_d_ok
    LD V8, 0
    JP ball_done_move
ball_d_ok:
    ADD V3, 1

ball_done_move:

    ; ── P1 paddle collision ──
    LD V9, V2
    SE V9, P1_HIT_X
    JP chk_p2_col

    LD V9, V0
    LD VA, V3
    SUBN V9, VA
    SE VF, 1
    JP chk_p2_col

    LD V9, V0
    ADD V9, PADDLE_H
    LD VA, V3
    SUBN V9, VA
    SE VF, 0
    JP chk_p2_col

    LD V7, 1
    JP ball_done_col

chk_p2_col:
    ; ── P2 paddle collision ──
    LD V9, V2
    SE V9, P2_HIT_X
    JP ball_done_col

    LD V9, V1
    LD VA, V3
    SUBN V9, VA
    SE VF, 1
    JP ball_done_col

    LD V9, V1
    ADD V9, PADDLE_H
    LD VA, V3
    SUBN V9, VA
    SE VF, 0
    JP ball_done_col

    LD V7, 0

ball_done_col:

    ; ── Scoring checks ──
    LD V9, V2
    SE V9, 0
    JP chk_right
    JP p2_scores

chk_right:
    SE V9, 63
    JP after_score
    JP p1_scores

p2_scores:
    ADD V5, 1
    JP score_done

p1_scores:
    ADD V4, 1

score_done:
    SE V4, MAX_SCORE
    JP chk_p2_won
    JP game_over_p1

chk_p2_won:
    SE V5, MAX_SCORE
    JP reset
    JP game_over_p2

reset:
    LD V2, BALL_START_X
    LD V3, BALL_START_Y
    RND V7, 1
    SE V7, 0
    JP reset_v7_1
    JP reset_vx_done
reset_v7_1:
    LD V7, 1
reset_vx_done:
    RND V8, 1
    SE V8, 0
    JP reset_vy_done
    LD V8, 1
reset_vy_done:

after_score:

    ; ── Delay ──
    LD VE, 2
    LD DT, VE
delay_wait:
    LD VE, DT
    SE VE, 0
    JP delay_wait

    JP game_loop

; ── Game over screens ───────────────────────────────────────────────────

game_over_p1:
    CLS
    LD V9, 1
    LD F, V9
    LD V9, 30
    LD VA, 13
    DRW V9, VA, 5
    JP go_wait

game_over_p2:
    CLS
    LD V9, 2
    LD F, V9
    LD V9, 30
    LD VA, 13
    DRW V9, VA, 5

go_wait:
    LD V9, K
    JP main
