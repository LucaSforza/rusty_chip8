#!/usr/bin/env python3
"""Flood-fill from player to show reachable area (the room)."""
import sys, json
from collections import deque

data = json.load(sys.stdin)
p = data["pixels"]

# Find player (same logic)
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

px, py = player[0], player[1]
print(f"Player at ({px},{py})")

# BFS flood-fill on walkable pixels
visited = set()
q = deque()
q.append((px, py))
q.append((px+1, py))
q.append((px, py+1))
q.append((px+1, py+1))
visited.update(q)

while q:
    x, y = q.popleft()
    for nx, ny in [(x-1,y), (x+1,y), (x,y-1), (x,y+1)]:
        if 0 <= nx < 64 and 0 <= ny < 32 and (nx, ny) not in visited:
            if p[ny][nx]:
                visited.add((nx, ny))
                q.append((nx, ny))

if not visited:
    print("No reachable area"); sys.exit(1)

xs = [v[0] for v in visited]
ys = [v[1] for v in visited]
print(f"Room size: {len(visited)} walkable pixels")
print(f"Bounds: x={min(xs)}-{max(xs)}, y={min(ys)}-{max(ys)}")

# Count exits (boundary walkable pixels touching black)
edge_count = 0
for x, y in visited:
    if x == 0 or x == 63 or y == 0 or y == 31:
        edge_count += 1
    else:
        for nx, ny in [(x-1,y), (x+1,y), (x,y-1), (x,y+1)]:
            if 0 <= nx < 64 and 0 <= ny < 32 and not p[ny][nx]:
                edge_count += 1
                break

print(f"Edge tiles (near wall): {edge_count}")

# Compact mini-map of the room
print("\nRoom map (█=walkable, ·=player, space=wall):")
min_x, max_x = min(xs), max(xs)
min_y, max_y = min(ys), max(ys)
for y in range(min_y, max_y + 1):
    row = ""
    for x in range(min_x, max_x + 1):
        if (px <= x <= px+1 and py <= y <= py+1) or \
           (px <= x <= px+1 and py <= y <= py+1 + 1):
            # Actually need exact check
            pass
        in_player = (px <= x <= px+1 and py <= y <= py+1)
        in_visited = (x, y) in visited
        if in_player:
            row += "·"
        elif in_visited:
            row += "█"
        else:
            row += " "
    print(f"{y:2d} {row}")
