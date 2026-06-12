#!/usr/bin/env python3
"""Check which directions are walkable from the player position."""
import sys, json

data = json.load(sys.stdin)
p = data["pixels"]

# Find player
player = None
for y in range(31):
    for x in range(63):
        if not (p[y][x] and p[y][x+1] and p[y+1][x] and p[y+1][x+1]):
            continue
        extra = sum(1 for dy in range(-1,3) for dx in range(-1,3)
                    if 0 <= x+dx < 64 and 0 <= y+dy < 32
                    and not (0 <= dx <= 1 and 0 <= dy <= 1)
                    and p[y+dy][x+dx])
        if player is None or extra < player[2]:
            player = (x, y, extra)
if player is None:
    print("Player not found"); sys.exit(1)

px, py, _ = player
print(f"Player at ({px},{py})")

# Check 4 directions: is there walkable path beyond the 2x2 block?
checks = [
    ("up",    px,   py-1, px,   py-2),
    ("down",  px,   py+2, px,   py+3),
    ("left",  px-1, py,   px-2, py),
    ("right", px+2, py,   px+3, py),
]
for name, x1, y1, x2, y2 in checks:
    if name in ("up", "down"):
        ok = (0 <= x1 < 63 and 0 <= y1 < 32 and p[y1][x1] and p[y1][x1+1])
    else:
        ok = (0 <= x1 < 64 and 0 <= y1 < 31 and p[y1][x1] and p[y1+1][x1])
    label = "FREE" if ok else "WALL"
    print(f"  {name}: {label}")
