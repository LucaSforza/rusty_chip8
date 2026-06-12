#!/usr/bin/env python3
"""Show full 16x8 tile map of the current board from pixel data.
Legend: ·=player  █=path  G=gate  ?=event  S=secret-wall  space=wall"""
import json, sys

data = json.load(sys.stdin)
p = data["pixels"]
v = data.get("v_regs", [0]*16)

manx = v[12] & 0x0F
many = v[11] & 0x07
boardno = v[13] & 0x0F

special_pos = [
    (3,6),(7,4),(14,6),(7,4),(3,3),(12,4),(7,5),(7,4),
    (7,6),(9,4),(10,4),(11,3),(5,3),(5,3),(5,3),(5,3),
]
event_tile = special_pos[boardno] if boardno < len(special_pos) else None

def tile_char(tx, ty):
    if tx == manx and ty == many:
        return "·"  # player marker
    px, py = tx * 4, ty * 4
    on = sum(1 for y in range(py, py+4) for x in range(px, px+4) if p[y][x])
    walkable = on >= 6
    if not walkable:
        return " "
    if boardno == 4 and 8 <= tx <= 12 and ty in (1,3):
        return "G"  # gate area
    if boardno == 6 and tx == 7 and ty >= 2:
        return "S"  # secret wall area
    if event_tile and tx == event_tile[0] and ty == event_tile[1]:
        return "?"
    return "█"  # path

print(f"Board {boardno:#x} Player ({manx},{many})")
print(f"  Legend: ·=player  █=path  G=gate  ?=event  S=secret  space=wall")
print(f"  Walkable tiles counted by ON pixels in 4x4 area (>=6 = path)")
print()

# Column headers
print("   " + "".join(f"{c:2d}" for c in range(16)))
for ty in range(8):
    row = "".join(f" {tile_char(tx, ty)} " for tx in range(16))
    print(f" {ty} {row}")
