#!/usr/bin/env python3
"""Craft the exchange gadget from its carried recipe in a real crafting table.

The harness opens a crafting table as a screen whose 3x3 grid is slots 100..108
(row-major) and whose result is 200. Called by `tools/e2e.sh` with the socket,
the control binary and the table's position.

Proves, in order:
  1. the carried `buildinggadgets2:gadget_exchanging` recipe — the jar's 3x3
     pattern `iri / dld / ili` with the `c:` tags resolved to concrete items
     (iron, redstone, diamond, lapis) — makes exactly one exchanging gadget
     and consumes every ingredient;
  2. the control: the same pattern without the lapis matches nothing, so the
     result slot stays empty. The positive craft is not a coincidence of the
     grid holding *some* item.

Exit 1 with the observed result on the first mismatch.
"""
from __future__ import annotations

import json
import subprocess
import sys

GRID = list(range(100, 109))
RESULT = 200
HOTBAR = 0
GADGET = "buildinggadgets2:gadget_exchanging"
# The recipe's `key` and `pattern`, row-major: iri / dld / ili.
PATTERN = {
    100: ("minecraft:iron_ingot", 4),
    101: ("minecraft:redstone", 1),
    102: ("minecraft:iron_ingot", 4),
    103: ("minecraft:diamond", 2),
    104: ("minecraft:lapis_lazuli", 2),
    105: ("minecraft:diamond", 2),
    106: ("minecraft:iron_ingot", 4),
    107: ("minecraft:lapis_lazuli", 2),
    108: ("minecraft:iron_ingot", 4),
}


def main() -> int:
    sock, ctl, x, y, z = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5]

    def run(*args: str) -> dict:
        out = subprocess.run([ctl, "--socket", sock, *args], text=True, capture_output=True)
        try:
            return json.loads(out.stdout)
        except json.JSONDecodeError:
            raise SystemExit(f"ctl {' '.join(args)}: {out.stdout}{out.stderr}")

    def wait(ticks: int = 4) -> None:
        run("wait", str(ticks))

    def slots() -> list[dict]:
        screen = run("screen")
        if "slots" not in screen:
            raise SystemExit(f"no menu is open: {screen}")
        return screen["slots"]

    def item_of(slot: int) -> str | None:
        found = [s for s in slots() if s["slot"] == slot]
        return (found[0].get("item") if found else None) or None

    def count_of(item: str) -> int:
        return run("inventory")["counts"].get(item, 0)

    def find(item: str) -> int:
        for slot in slots():
            if slot.get("item") == item:
                return slot["slot"]
        raise SystemExit(f"missing ingredient {item} (inventory: {run('inventory')['counts']})")

    def place(item: str, targets: list[int]) -> None:
        """Pick the item up, put one in each target, put the rest back."""
        slot = find(item)
        run("click-slot", str(slot))
        wait(1)
        for target in targets:
            run("click-slot", str(target), "right")
            wait(1)
        run("click-slot", str(slot))
        wait(1)

    def clear_table() -> None:
        try:
            run("close")
        except SystemExit:
            pass
        wait(3)
        run("setblock", x, y, z, "minecraft:air")
        run("setblock", x, y, z, "minecraft:crafting_table")
        wait(2)
        run("command", f"tp @s {float(x) - 0.5} {y} {float(z) + 0.5}")
        wait(3)

    def open_table() -> None:
        run("interact", x, y, z)
        wait(5)

    def fill(enough: bool) -> None:
        clear_table()
        run("command", "clear @s")
        wait(2)
        for item, count in sorted(set(PATTERN.values())):
            run("give", item, str(count))
        wait(3)
        open_table()
        for item, _count in sorted(set(PATTERN.values())):
            targets = [slot for slot, (want, _n) in PATTERN.items() if want == item]
            if enough or item != "minecraft:lapis_lazuli":
                place(item, targets)

    # 1. The recipe as it declares.
    fill(enough=True)
    result = item_of(RESULT)
    if result != GADGET:
        raise SystemExit(f"the carried recipe's result is {result!r}, expected {GADGET}")
    found = [s for s in slots() if s["slot"] == RESULT][0]
    if found.get("count") not in (1, None):
        raise SystemExit(f"the recipe made {found.get('count')} gadgets, expected 1")
    run("click-slot", str(RESULT))
    wait(2)
    run("click-slot", str(HOTBAR))
    wait(2)
    made = count_of(GADGET)
    if made != 1:
        raise SystemExit(f"the crafted gadget is not in the inventory (count {made})")
    for item, _count in sorted(set(PATTERN.values())):
        left = count_of(item)
        if left != 0:
            raise SystemExit(f"the recipe left {left} {item}, expected every ingredient consumed")
    run("close")
    wait(3)
    print("  the carried exchanging recipe crafted one gadget and consumed iron x4, redstone x1, diamond x2, lapis x2")

    # 2. The control: the same pattern without the lapis matches nothing.
    fill(enough=False)
    result = item_of(RESULT)
    if result is not None:
        raise SystemExit(f"the pattern without lapis made {result!r}, expected nothing")
    print("  the same pattern without the lapis leaves the result empty")
    run("close")
    wait(3)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
