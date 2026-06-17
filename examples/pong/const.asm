; Pong — game constants and structure

; Screen
.const SCREEN_W = 64
.const SCREEN_H = 32

; Paddle
.const PADDLE_X1 = 4
.const PADDLE_X2 = 58
.const PADDLE_H = 6
.const PADDLE_SPEED = 1
.const MAX_PADDLE_Y = 26

; Ball
.const BALL_SPEED = 1
.const BALL_START_X = 32
.const BALL_START_Y = 16

; Scoring
.const MAX_SCORE = 5
.const SCORE_X1 = 25
.const SCORE_X2 = 37
.const SCORE_Y = 1

; Game modes
.const MODE_1P = 1
.const MODE_2P = 2

; Key bindings (CHIP-8 hex keypad)
.const KEY_P1_UP = 0x5   ; W
.const KEY_P1_DN = 0x8   ; S
.const KEY_P2_UP = 0x6   ; E
.const KEY_P2_DN = 0x9   ; D
.const KEY_1 = 0x1
.const KEY_2 = 0x2

; Collision trigger X positions
.const P1_HIT_X = 6       ; PADDLE_X1 + 2
.const P2_HIT_X = 57      ; PADDLE_X2 - 1

; Structure for documentation
struct GameState {
    p1_y byte
    p2_y byte
    ball_x byte
    ball_y byte
    p1_score byte
    p2_score byte
    mode byte
    ball_dir_x byte
    ball_dir_y byte
}
