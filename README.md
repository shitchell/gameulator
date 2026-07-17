# Gameulator

Tooling to parse, sync, and explore emulator save + ROM data, starting with
Pokémon Yellow Legacy. It reads a Gen-1 `.sav` into a fully-typed model
(verified against the disassembly), resolves species/move/item names through a
generated game-data overlay, and renders the party, bag, PC, and save info
through a small CLI. It's a Rust monorepo (Cargo workspace).

## Layout

Crates live under `crates/`:

- **`pokegen1`** — Gen-1 save/ROM parsing. Provides the structural `Model`
  (party, bag, PC, trainer, playtime, checksum, etc.) plus the Yellow Legacy
  overlay (generated name tables for species/moves/items and Legacy-specific
  constants).
- **`app`** — controller layer. Turns parsed model data into presentation-ready
  view DTOs (party summaries, item views, save info) with structured status
  conditions.
- **`cli`** — the `gameulator` binary. A thin front-end over `app` that renders
  the views as text or JSON.
- **`xtask`** — dev task runner. Regenerates the game-data tables (name tables,
  constants) from the pinned disassembly; run it after a ROM-version bump.

Design and milestone plan live in `docs/plans/`:

- `docs/plans/2026-07-17-gameulator-design.md` — overall design.
- `docs/plans/2026-07-17-gameulator-milestone1-plan.md` — Milestone 1 plan.

## Build the ROM

The Yellow Legacy ROM is built from the pinned disassembly submodule
(`vendor/pokemon-yellow-legacy`):

```sh
make rom
```

This **requires RGBDS 0.6.1** — newer versions break the disassembly build, so
the `rom` target preflights the version and fails with a clear message
otherwise. The toolchain path defaults to a machine-specific location
(`RGBDS ?= $(HOME)/.local/src/rgbds/`, trailing slash required) and is
overridable:

```sh
make rom RGBDS=/path/to/rgbds/
```

The built ROM is copied into `games/Pokemon/Yellow Legacy/rom/` (gitignored).

## Regenerate game data

After a ROM-version bump, regenerate the overlay name tables and constants from
the disassembly:

```sh
cargo run -p xtask
```

## Use the CLI

The binary is installed as `gameulator`. During development, run it through
Cargo:

```sh
cargo run -p cli -- party <save.sav>     # show the party
cargo run -p cli -- bag <save.sav>       # show the bag items
cargo run -p cli -- pc <save.sav>        # show the PC items
cargo run -p cli -- info <save.sav>      # trainer, playtime, checksum
```

Output flags (on the view commands):

- `--json` — emit pretty JSON instead of text.
- `--compact` — one diff-friendly line per entry.

## Status

This repo currently delivers **Milestone 1**: the save parser plus the
`gameulator` CLI. It builds the Yellow Legacy ROM, parses a save into a typed
model verified against the disassembly, resolves names via the generated
overlay, and renders party/bag/pc/info.

Not yet implemented (scoped to later milestones or deferred):

- **Save sync** — Syncthing-based save synchronization (Milestone 2).
- **Web view** — a Leptos/WASM browser UI (Milestone 3).
- **Type-coverage / TypeChart** — no type-effectiveness data yet.
- **Dex progress** — no Pokédex seen/owned progress reporting.
- **Save diff** — no structured diff between two saves.
- **Badges in `Save`** — badge state is not surfaced in the model.
- **Move max-PP** — `MoveView` carries the current PP only; `read_save.py`
  exposed both `pp` and `maxpp`. Restoring max-PP needs a base-PP table added to
  the overlay.
- **`wDifficulty` from the save** — the difficulty flag is not yet read from
  save data (the overlay knows the Normal/Hard constants, but the save value
  isn't parsed).

Milestones 2 (Syncthing-based save sync) and 3 (Leptos/WASM web view) get their
own plans later.
