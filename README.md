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

## Sync (Milestone 2)

Save sync mirrors your Yellow Legacy battery `.sav` between phone and computer
over the LAN via **Syncthing** (manual setup, no cloud), and the
`gameulator-sync` watcher then validates each incoming save (checksum),
snapshots every valid write (keep-all history under `saves/snapshots/`), alarms
on a stale-device playtime regression, and writes a parsed `saves/status.json`
summary (the seam the Milestone-3 web view will read). The sync unit is one
in-game SAVE — Pizza Boy's battery `.sav`; save-states are not synced.

Run the watcher:

```sh
cargo run -p sync --bin gameulator-sync -- --saves-dir "games/Pokemon/Yellow Legacy/saves"
```

Full setup and operator notes: **[`docs/SYNCTHING.md`](docs/SYNCTHING.md)**.

## Web dashboard (Milestone 3)

A live browser dashboard (Leptos/WASM) showing the current save — trainer,
playtime, and the party (HP bars, stats, moves, and status conditions), plus the
bag and PC. It reuses the `app` view DTOs, so what the dashboard shows matches
the CLI. The frontend **polls** `status.json` (the file `gameulator-sync` writes)
every ~2 s and re-renders, so a save synced from your phone shows up on its own.

### Prerequisites

```sh
rustup target add wasm32-unknown-unknown   # WASM build target
cargo install trunk                        # the WASM bundler
```

**A `status.json` must exist first.** Run `gameulator-sync` (or the sync
pipeline) against your saves dir so it writes one — a fresh clone's
`games/Pokemon/Yellow Legacy/saves/status.json` is gitignored/absent, so until
then the dashboard just shows a "waiting for a save" message.

### Run it

```sh
make web
```

This builds the release WASM frontend and runs the server; then open
**http://localhost:8770**. Or do the two steps manually:

```sh
cd crates/web && trunk build --release
cargo run -p web-server --bin gameulator-web   # run from the repo root
```

Run the server from the repo root — its default paths (`--dist-dir`,
`--status-path`) are repo-root-relative.

### Localhost-only

The server binds `127.0.0.1`, so the dashboard is **not exposed to the LAN**.
There's no auth on it — keeping it localhost-only is the secure default. (A
future `--bind`/`--host` flag would enable opt-in LAN access, e.g. to view the
dashboard from a phone.)

### Theming / composability

All styling flows through a single CSS design-token block in
[`crates/web/style.css`](crates/web/style.css) — that `:root` block is the one
restyle point. Components carry only classes, never colors, so changing a token
re-themes everything. Light/dark **follows the system theme** automatically: a
`prefers-color-scheme: dark` block overrides the same tokens.

### Layout note

`crates/web` is the WASM frontend; `crates/web-server` is the axum backend (the
`gameulator-web` bin). `web` is deliberately kept out of the workspace's
`default-members` as a **speed/isolation convention** — so a bare `cargo build`
or `cargo test` skips recompiling the wasm crate — **not** because it can't build
natively (`cargo test --workspace` still works). In practice:

- Native suite: `cargo test --workspace --exclude web`
- Frontend check: `cargo check -p web --target wasm32-unknown-unknown` + `trunk build`

The release wasm bundle is ~420 KB.

## Status

**Milestone 1** delivers the save parser plus the `gameulator` CLI. It builds
the Yellow Legacy ROM, parses a save into a typed model verified against the
disassembly, resolves names via the generated overlay, and renders
party/bag/pc/info.

**Milestone 2 (save sync) is partial:** the Rust `gameulator-sync` watcher
(validate / snapshot / regression-guard / `status.json`) is **done and tested**;
Syncthing itself is a **documented manual setup** (see
[`docs/SYNCTHING.md`](docs/SYNCTHING.md)), not code.

**Milestone 3 (web dashboard) is done:** a live-polling Leptos/WASM dashboard
over `status.json` — composable party/info/item components, a single token-CSS
design system (the one restyle point), and system-following light/dark. Run it
with `make web` (see [Web dashboard](#web-dashboard-milestone-3) above).

Not yet implemented (scoped to later milestones or deferred):

- **WebSocket push** — the dashboard **polls** `status.json` (~2 s); a push
  channel (server-notifies-on-change) is deferred. Poll is the v1.
- **Manual light/dark toggle** — theme **follows the system** (`prefers-color-scheme`);
  an in-page toggle is deferred. System-follow is the v1.
- **Dex grid / type-coverage / run timeline** — richer dashboard views need data
  domains not yet extracted from the ROM/save (Pokédex progress,
  type-effectiveness, run history); these belong to the library-expansion
  roadmap.
- **Client-side WASM `.sav` parsing** — the v1 frontend reads the already-parsed
  `status.json`; parsing raw `.sav` bytes in the browser is deferred.
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
- **`status.json` `snapshot` is an absolute path** — the watcher writes the
  full snapshot path; Milestone 3 likely wants a relative filename instead.
- **`Deserialize` on the app DTOs** — `StatusView` and its `app` DTOs are
  `Serialize`-only today; add `Deserialize` only if the WASM view reconstructs
  `StatusView` (rather than reading raw JSON).
- **`status.json` self-heal** — a partial-write failure needs no rollback: the
  snapshot is durable and `status.json` self-heals on the next accepted save.
