# Gameulator — notes for Claude

Tooling to parse, sync, and explore emulator save/ROM data — starting with **Pokémon
Yellow Legacy**. Rust monorepo. Read `docs/ROADMAP.md` first — it's the north-star
vision (structured ground-truth access to *everything* in the ROM+save so a user can
compute anything) and the data-sourcing design.

## Architecture (layers)

- **`pokegen1`** — the Model. `core::` = game-agnostic Gen-1 save parsing + mechanics +
  the trait seam (`core::data`: `SpeciesTable`/`MoveTable`/`ItemTable` → `GameData`).
  `games::yellow_legacy` = the overlay: data tables **generated from the pinned
  disassembly** + Legacy constants. Stays **table-free** in `core`.
- **`app`** — the Controller. Presentation-agnostic ops (`load_save`, `party_summary`,
  `items_view`, `save_info`) returning serde **view DTOs**. Owns name resolution,
  nickname suppression, `fainted` materialization, and `game_data(GameId) -> Box<dyn
  GameData + Send>` (the single game-selection chokepoint). `StatusView` (the web↔sync
  contract) lives here. Re-exports the pokegen1 types its public API names.
- **Views** — `cli` (the `gameulator` binary), `web` (Leptos/WASM frontend) +
  `web-server` (axum `gameulator-web`). Views only **format** DTOs — no logic.
- **`sync`** — the M2 save-sync watcher (`gameulator-sync`): validate → snapshot →
  playtime-regression guard → write `status.json`.
- **`xtask`** — regenerates the game-data JSON from the disassembly (`cargo run -p xtask`).

## Governing conventions (follow these)

- **Locality:** "one conceptual change → one contiguous edit block." (The user's #1 rule.)
- **Disassembly = ground truth.** Never hardcode a value a ROM-version bump could move;
  regenerate it from the pinned `vendor/pokemon-yellow-legacy` (tag V1.0.10) via `xtask`.
- **New capability = a NEW trait**, never new methods on the existing name-lookup traits —
  keeps name-only consumers from depending on stat/map resolution. Core stays table-free.
- **Web styling = the CSS token block** (`crates/web/style.css`) is the ONE restyle point;
  components carry only class names, never colors/sizes. Light/dark follows the system via
  a single `prefers-color-scheme` override of the same tokens.
- **Commit on `main` directly** (personal repo, no per-task branches). Commits are
  GPG-signed via the git wrapper.
- Built via superpowers **subagent-driven-development** (implementer + spec-review +
  quality-review per task). Plans + designs live in `docs/plans/`.

## Build / test / run

- **Native suite:** `cargo test --workspace --exclude web` (the `web` crate is wasm; it's
  in `members` but out of `default-members` as a speed/isolation convention — `--workspace`
  still *works*, `--exclude web` just skips recompiling it on the host).
- **Frontend:** `cargo check -p web --target wasm32-unknown-unknown` + `cd crates/web &&
  trunk build` (needs `rustup target add wasm32-unknown-unknown` + `cargo install trunk`).
- **ROM:** `make rom` (RGBDS **exactly 0.6.1**). **Web dashboard:** `make web` →
  localhost:8770. **Sync:** `cargo run -p sync --bin gameulator-sync`.
- Keep `cargo clippy --workspace --all-targets` + `cargo fmt --all --check` clean.

## Gotchas worth knowing

- Party level is at struct offset **0x21**, NOT 0x03 (0x03 is the boxed-format level).
- The SRAM checksum + all offsets were verified against the disassembly source, not memory.
- Real saves/ROMs live under gitignored `games/`. **`personal/` is gitignored — never
  commit it** (the user's run notes/journey; kept out of the public repo, purged from
  history).
- `reference/` holds the original Termux parsing scripts + hard-won lessons (portable only;
  personal notes were removed).

## Status & what's next

- **M1 (done):** save parser (`pokegen1`) + `app` controller + `gameulator` CLI.
- **M2 (done):** the `gameulator-sync` watcher (Syncthing is documented manual setup —
  `docs/SYNCTHING.md`).
- **M3 (done):** the live Leptos/WASM web dashboard over `status.json`.
- **NEXT — library expansion** toward `docs/ROADMAP.md`: Layer-1 data-domain traits + xtask
  extractors (candidate first: Pokédex seen/owned flags; then base-stats/moves/type-chart/
  learnsets → a battle solver; then map extraction), and Layer-2 analysis crates. Settle the
  cache schema + the config-ROM > cached-JSON > bundled-ROM source resolution (at the
  `app::game_data` chokepoint) when building it.
- **Open web polish (a tweak-session):** dark-mode pastel-badge contrast; optional
  `[data-theme]` toggle; a staleness indicator from `StatusView.last_change`.
