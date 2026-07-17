# Gameulator — Design (Pokémon Yellow Legacy first)

> Status: **validated in brainstorming, ready for implementation planning**
> Date: 2026-07-17
> Scope: directory/repo organization, a modular Gen-1 parsing lib, an MVC (CLI +
> web) utility atop it, phone↔computer save sync, and ROM acquisition — starting
> with **Pokémon Yellow Legacy v1.0.10**, structured to expand to other Gen-1
> games (Red/Blue/Yellow-vanilla) and other emulator games later.

---

## 1. Purpose & context

The user is mid-run through Pokémon Yellow Legacy on Pizza Boy (Android), past the
Elite Four with an L100 Mewtwo + ~L60–70 party. Goals:

- **Seamlessly swap** between phone and computer by syncing the `.sav` (near-realtime
  in the ideal case).
- Grow a set of Termux tracking/diff/stats scripts into a **full parser/explorer** for
  both the ROM and the save — from a fresh run to a full 151-caught completion.
- Organize this directory to host **multiple games/emulators** over time, with some
  code living in one or more repos.

### Governing principle (user's, direct quote)

> "if there is a decent chance we might go in and change some single conceptual
> behavior/ui later, we should ideally only have to change one contiguous block of
> code."

Plus: **compiled/strongly-typed** languages and **modular/flexible** architecture for
AI-driven dev. Every decision below is filtered through this locality-of-change lens.

---

## 2. Key decisions (with rationale)

| Topic | Decision | Status | Rationale |
|---|---|---|---|
| Language | **Rust** | Accepted | Best-in-class binary parsing, strong compiler guardrails for AI-driven work, and one lib compiles **native (CLI) + WASM (web)** — parsing logic written once. User: prefers "strongly typed / compiled … far more for AI-driven projects." |
| Repo topology | **One monorepo**, Cargo workspace | Accepted | Keeps DRY refactors atomic; ROMs/saves gitignored. Split crates out later if a lib matures. |
| Lib crate split | **Start with ONE lib crate**, strong internal module seams; extract `gen1-core` only when a 2nd game proves the seam | Accepted | User: "we won't necessarily know how gen1-core should be split from the per-version overlays until we explore red, blue, yellow vanilla…" Premature crate boundaries are the trainwreck; moving modules within a crate is trivial. |
| Data source of truth | **The disassembly**, per ROM version; generate data tables from it | Accepted | Portable notes: "treat the disassembly of the player's exact ROM version as ground truth, regenerate the data model from it, and don't hardcode values that a version bump could move." (Thunder PP, evo levels, trainer rosters, level caps all drift.) |
| Web tech | **WASM-first**, all-Rust (Leptos), axum as thin host | Accepted | User wanted WASM sooner, not deferred: "i would defer WASM too much. sooner the better." All-Rust keeps analysis logic shared with the CLI. |
| Sync transport | **Syncthing** (LAN, bidirectional) + a validating watcher on top | Accepted | Near-realtime, no cloud, free conflict detection. User is on Termux + same LAN. |
| Concurrency safety | Soft **active-device marker + monotonic-playtime regression alarm** (no hard lock) | Accepted | User: "i'd love a lock, but if it can't be cleanly/elegantly solved, i can settle for 'i promise myself i won't play on two devices at once'." Regression detection leverages the lib and needs no fragile distributed lock. |
| ROM acquisition | **Build from source** (RGBDS 0.6.1, `make`), pinned to **v1.0.10** submodule | Accepted | No copyrighted base ROM needed; same checkout doubles as the data-generation source. |

---

## 3. Directory & repo organization

```
gameulator/                        # ONE monorepo (git), Cargo workspace
├── crates/
│   ├── pokegen1/                  # THE lib (Model). Internal seams:
│   │   │                          #   core::   parsing, mechanics, traits
│   │   └                          #   games::yellow_legacy::  tables + overrides
│   ├── app/                       # Controller: presentation-agnostic operations
│   │                              #   (load_save, party_summary, diff, coverage…)
│   ├── cli/                       # View: clap; --json on everything
│   ├── web/                       # View: Leptos (WASM) frontend + axum host
│   └── sync/                      # Syncthing watcher: validate/snapshot/regression
├── vendor/
│   └── pokemon-yellow-legacy/     # pinned git submodule @ v1.0.10 tag
│                                  #   → builds pokeyellow.gbc AND feeds data-gen
├── games/                         # GITIGNORED — binaries + personal data
│   └── Pokemon/Yellow Legacy/
│       ├── rom/                   # built .gbc lands here
│       ├── saves/                 # the live .sav syncs here (Syncthing)
│       └── snapshots/             # timestamped save history (auto, diffable)
├── reference/                     # Termux scripts + hard-won parsing notes (tracked)
├── docs/plans/                    # design docs
└── Cargo.toml                     # workspace manifest
```

**Tracked:** code, `reference/`, the submodule *pointer*. **Gitignored:** `games/`
(ROMs, saves, snapshots), built artifacts, the submodule's blobs.

Dependency flow: `pokegen1 → app → { cli (native), web (WASM) }`; `sync` standalone,
feeding `web` change events.

---

## 4. The lib (`pokegen1`) — the Model

**Two data sources, cleanly separated:**
- **Static game data** — derived *from the disassembly* (base stats, movesets, type
  chart, TM/HM lists, trainer/boss parties, evo methods). A build step (`xtask`) parses
  the submodule's `.asm`/data files and emits generated tables (JSON/RON). This is the
  Rust successor to `reference/parse_legacy.py`. Never hand-edited; regenerate on version
  bump.
- **Dynamic player state** — parsed from the `.sav` (party, boxes, bag, badges, money,
  location, playtime, `wDifficulty`). Rust successor to `reference/read_save.py`.

**Internal structure (one crate, seams ready to split):**
- `core::` — SRAM offset map, 44-byte party struct, `gbstr` charmap decode, item-list
  walker, **SRAM checksum**, domain types (`Save`, `Pokemon`, `Move`, `Species`, `Bag`,
  `Pokedex`), generic mechanics (stat/damage/crit formulas, badge-boost model), and
  **traits** for anything a variant overrides (`TypeChart`, `SpeciesTable`, `MoveTable`).
- `games::yellow_legacy::` — the generated tables + Legacy overrides: Ghost = special
  category, **Pikachu badge boost ×1.25** (others ×1.125), `wDifficulty` branches
  (1/256 miss kept on hard, level caps hard-only), **bag capacity 41**.

Consumers talk to **traits**, never a concrete table; static data lives in **data files**,
not hardcoded Rust — so relocating a table is moving a file, and swapping a game is
implementing a trait set. The `Save` + game-data tables join in the lib (species id →
base stats/type → effective in-battle stats).

**Difficulty is load-bearing:** surface `wDifficulty` prominently; damage/reliability
calcs branch on it.

---

## 5. The MVC utility

- **Model** — `pokegen1` (§4).
- **Controller** — `app` crate: presentation-agnostic ops returning plain structs
  (`load_save`, `party_summary`, `diff(a,b)`, `type_coverage`, `dex_progress`,
  `boss_matchup`, `box_search`). **All real logic lives here, once.** Compiles to native
  and WASM.
- **Views** — thin renderers over the controller:
  - **CLI** (`clap`): tables/text with `--json` on everything (mirrors the existing
    `--compact`/`--json`). For quick lookups, scripting, and Claude's inspection.
  - **Web** (Leptos → WASM): dex grid, coverage heatmap, party/box cards, run timeline —
    Rust components calling the *same* controller fns as the CLI. **axum** serves the SPA
    assets + exposes the live synced `.sav` bytes; the WASM frontend fetches + parses
    client-side.

---

## 6. Sync system

**Transport: Syncthing.** Phone runs the **Syncthing Android app** (F-Droid — reliable
background sync); computer runs the daemon; they share `ROMs/saves/` ↔
`games/Pokemon/Yellow Legacy/saves/`. LAN-direct, bidirectional, encrypted, no cloud.

**Granularity:** Pizza Boy writes the `.sav` only on an **in-game SAVE** (battery save);
save-states are a separate format we deliberately do **not** sync. So the sync unit is
"per in-game save" — the correct handoff unit. Feel: SAVE on phone → ~1s later on
computer.

**`sync` crate layers logic over Syncthing's raw mirror:**
1. **Watch** the synced `.sav` (inotify).
2. **Validate** via the lib's SRAM **checksum** — quarantine a half-written/corrupt file.
3. **Snapshot** to `saves/snapshots/<timestamp>.sav` — automatic `pokedump` history.
4. **Emit** a "save changed" event → WebSocket to `web` (realtime dashboard).

**Soft lock (no hard lock):** the lib reads **playtime**, so the watcher does
**monotonic-playtime / regression detection** — an incoming `.sav` with *less* playtime
(or fewer badges/events) than the latest snapshot flags "you played the stale device"
and preserves the newer snapshot instead of clobbering. Combined with Syncthing's
`.sync-conflict` files, that's real protection over the honor-system promise.

---

## 7. ROM acquisition

**Build from source** (recommended, and doubles as the data source):
```
git submodule add https://github.com/cRz-Shadows/Pokemon_Yellow_Legacy vendor/pokemon-yellow-legacy
cd vendor/pokemon-yellow-legacy && git checkout <v1.0.10 tag>
# RGBDS EXACTLY 0.6.1 (newer breaks); then:
make            # → pokeyellow.gbc  (no copyrighted base ROM needed)
```
Output `pokeyellow.gbc` → `games/Pokemon/Yellow Legacy/rom/`. Pin the submodule to the
**v1.0.10** tag so ROM + generated data + the save all agree by construction. Version
bump = re-checkout tag → regenerate data → rebuild, one lever.

(Alternative, not chosen: apply the official **IPS patch** (v1.0.10, base ROM SHA-1
`cc7d0326…`) to a clean Yellow ROM — needs a base ROM and gives no source tree.)

Refs: <https://github.com/cRz-Shadows/Pokemon_Yellow_Legacy> ·
INSTALL.md · Releases.

---

## 8. Reference material (in `reference/`)

From the prior phone Claude session (hard-won parsing lessons):
- `read_save.py` / `read_party.py` — Gen-1 SRAM parsing (party, bag, PC, playtime).
  **Gotcha:** party level is at struct offset **0x21** (`read_save.py`), not 0x03
  (`read_party.py`'s boxed-format offset). Trust 0x21 for party.
- `parse_legacy.py` — parses the disassembly into `legacy.json` (type chart, moves,
  pokémon, boss rosters). Prototype for the Rust `xtask` data generator.
- `pokedump.sh` — the "SAVE in-game → dump → diff vs last → clipboard" workflow the
  `sync` snapshotting automates.
- `POKEMON_YELLOW_LEGACY_REFERENCE.md`, `YELLOW_LEGACY_PORTABLE_NOTES.md`,
  `pokemon_tracker.md`, `PLAYER_PROFILE_AND_JOURNEY.md`, plus `.xlsx` models.

Key SRAM offsets (Gen-1 SRAM bank 1): name `0x2598`, bag `0x25CA`, PC items `0x2834`,
playtime H/M `0x2CED`/`0x2CEF`, party count `0x2F2C`, party data `0x2F34` (44-byte
structs), OT names `0x2F9C`, nicknames `0x307E`.

---

## 9. Open items / not-yet-decided

- Exact `pokegen1` module API (trait signatures) — settle during implementation.
- Leptos component structure and the axum ↔ WASM save-bytes endpoint shape.
- Whether the data generator emits JSON, RON, or `build.rs`-generated Rust.
- Snapshot retention/pruning policy.
- The active-device marker's exact claim mechanism (phone shortcut vs desktop toggle).

---

## 10. Proposed implementation order (draft)

1. Scaffold monorepo + workspace; `git init`; add `vendor/` submodule @ v1.0.10; `.gitignore`.
2. Build the ROM (RGBDS 0.6.1 → `pokeyellow.gbc`) to prove the toolchain.
3. `pokegen1`: port `read_save.py` → Rust save parser + checksum + domain types (test
   against the user's real `.sav`).
4. Data generator (`xtask`) from the submodule → generated Legacy tables.
5. `app` controller ops + `cli` view (reach parity with the Termux scripts).
6. `sync`: Syncthing setup + watcher (validate/snapshot/regression).
7. `web`: Leptos + axum, wired to controller + sync change events.
```
```
