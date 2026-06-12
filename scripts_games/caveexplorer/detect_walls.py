#!/usr/bin/env python3
"""Detect overworld player from registers (Vc=manx, Vb=many) and check walls.
Also detects special walls: gates, secret walls, event tiles."""
import json, sys

data = json.load(sys.stdin)
p = data["pixels"]
v = data.get("v_regs", [0]*16)

manx = v[12] & 0x0F
many = v[11] & 0x07
boardno = v[13] & 0x0F

print(f"Board {boardno:#x} Player tile ({manx}, {many})")

def tile_walkable(tx, ty):
    if not (0 <= tx < 16 and 0 <= ty < 8):
        return False
    px, py = tx * 4, ty * 4
    on = sum(1 for y in range(py, py+4) for x in range(px, px+4) if 0 <= x < 64 and 0 <= y < 32 and p[y][x])
    return on >= 6

edges = {"north": many == 0, "south": many == 7, "west": manx == 0, "east": manx == 15}
for name, dx, dy in [("north", 0, -1), ("south", 0, 1), ("west", -1, 0), ("east", 1, 0)]:
    if edges[name]:
        print(f"  {name}: EDGE (board transition)")
    else:
        free = tile_walkable(manx + dx, many + dy)
        print(f"  {name}: {'FREE' if free else 'WALL'}")

# Special wall detection
notes = []

# Secret wall: board 6, column 7, rows 2-7 become walkable after rumble event
if boardno == 6 and manx == 7:
    notes.append("Secret wall: rows 2-7 may open here after rumble event")

# Gates: board 4, columns 8-12 have gate sprites (path underneath always walkable)
if boardno == 4 and 8 <= manx <= 12:
    notes.append("Gate area: path beneath gate is already walkable")

# Special positions (from game source code)
special_pos = [
    (3,6),(7,4),(14,6),(7,4),(3,3),(12,4),(7,5),(7,4),
    (7,6),(9,4),(10,4),(11,3),(5,3),(5,3),(5,3),(5,3),
]
sp = special_pos[boardno] if boardno < len(special_pos) else None
if sp and manx == sp[0] and many == sp[1]:
    notes.append("Event tile: interacting here triggers a special event")

if notes:
    print("\nNotes:")
    for n in notes:
        print(f"  {n}")
