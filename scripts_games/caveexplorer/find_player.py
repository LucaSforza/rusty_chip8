#!/usr/bin/env python3
"""Find the player (2x2 block) in CHIP-8 display pixels (64x32)."""
import sys, json

data = json.load(sys.stdin)
p = data["pixels"]  # 32x64 bool, p[y][x]

candidates = []
for y in range(31):
    for x in range(63):
        if not (p[y][x] and p[y][x+1] and p[y+1][x] and p[y+1][x+1]):
            continue
        # Count ON neighbors outside the 2x2 core
        extra = 0
        for dy in range(-1, 3):
            for dx in range(-1, 3):
                nx, ny = x + dx, y + dy
                if 0 <= nx < 64 and 0 <= ny < 32:
                    if not (0 <= dx <= 1 and 0 <= dy <= 1):  # outside core
                        if p[ny][nx]:
                            extra += 1
        candidates.append((x, y, extra))

if not candidates:
    print("Player not found")
    sys.exit(1)

# Most isolated 2x2 = player
px, py, _ = min(candidates, key=lambda c: c[2])
print(f"Player at ({px}, {py})")
print(f"Bounds: x={px}-{px+1}, y={py}-{py+1}")
