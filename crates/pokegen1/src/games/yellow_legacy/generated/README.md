# Generated Yellow Legacy name tables

**These files are GENERATED — do NOT hand-edit.**

`species.json`, `moves.json`, and `items.json` are produced from the pinned
Pokémon Yellow Legacy disassembly (`vendor/pokemon-yellow-legacy`, tag
**V1.0.10**) by the `xtask` dev tool:

```sh
cargo run -p xtask
```

Each file is a JSON object mapping a stringified id to a display name, e.g.
`{"131": "MEWTWO", ...}`. Keys are in numeric order for stable diffs.

## What each table contains

- **`species.json`** — internal-id -> species name (1..190). Keyed by the
  **internal** id used in save files (RHYDON=1 … MEWTWO=131 … VICTREEBEL=190),
  **not** Pokédex number. MISSINGNO. gap entries are present at their real
  internal ids. Source: `data/pokemon/names.asm` (positional).
- **`moves.json`** — move id -> name (1..165). Source: `data/moves/names.asm`
  (positional). e.g. `85` -> `THUNDERBOLT`.
- **`items.json`** — item id -> name. Regular bag items 1..83 come from
  `data/items/names.asm` (positional, stopping at the `NUM_ITEMS` assert so the
  elevator-floor pseudo-items are excluded). HM items 196..200 and TM items
  201..250 are derived from the `add_hm` / `add_tm` ordering in
  `constants/item_constants.asm`, named `HM-<Move>` / `TM-<Move>`
  (e.g. `196` -> `HM-Cut`, `201` -> `TM-Mega Punch`).

## Provenance / regeneration

The disassembly is ground truth. **Regenerate these files after any ROM-version
bump** by re-running `cargo run -p xtask`. Names preserve the disassembly's
original casing and special characters (e.g. `POKé BALL`, `NIDORAN♂`,
`MR.MIME`), which may differ from the ASCII-only strings in
`reference/read_save.py`.
