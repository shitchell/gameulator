# Gameulator Milestone 1 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (or
> superpowers:subagent-driven-development) to implement this plan task-by-task.
> Companion design: `docs/plans/2026-07-17-gameulator-design.md`.

**Goal:** Stand up the Rust monorepo, build the Yellow Legacy ROM from source, and
reach feature parity with the existing Termux `.sav` scripts — a Rust `pokegen1` lib +
`app` controller + `cli` that parse a Yellow Legacy save and print party/bag/PC/stats,
with game-data tables generated from the pinned disassembly.

**Architecture:** One Cargo workspace. `pokegen1` (Model) holds `core::` parsing +
mechanics + traits and `games::yellow_legacy::` generated tables + overrides. `app`
(Controller) exposes presentation-agnostic ops. `cli` (View) renders them. A pinned
`vendor/pokemon-yellow-legacy` git submodule (v1.0.10) builds the ROM AND feeds an
`xtask` data generator. Everything trait-mediated and data-driven so the core/overlay
seam can move later without a rewrite.

**Tech Stack:** Rust (stable), Cargo workspace, `clap` (CLI), `serde`/`serde_json`
(data tables + `--json`), RGBDS 0.6.1 (ROM build), git submodule.

**Governing principle (user):** "if there is a decent chance we might go in and change
some single conceptual behavior/ui later, we should ideally only have to change one
contiguous block of code." Compiled + modular everywhere.

**Skills to use during execution:**
- @superpowers:test-driven-development — every parsing task is test-first.
- @superpowers:systematic-debugging — when a parse disagrees with a known-good value.
- @superpowers:verification-before-completion — run the command, show output, before
  claiming a task done.

---

## Conventions

- **TDD always.** Write the failing test, run it red, minimal impl, run it green, commit.
- **Test fixtures for the parser are synthetic**: build a small in-memory SRAM byte
  buffer in the test with known values at known offsets (no copyrighted data). Offsets
  come from the design doc §8 and `reference/read_save.py`.
- **One real-save integration test** reads `games/Pokemon/Yellow Legacy/saves/*.sav` if
  present and asserts it parses without error; **skips** (not fails) when absent, so the
  suite is green before the save is synced.
- **Commit after every green step.** Conventional-commit messages.
- Run tests with `cargo test -p <crate>`; run the CLI with `cargo run -p cli -- …`.

---

## Task 1: Bootstrap the repo & workspace

**Files:**
- Create: `.gitignore`, `Cargo.toml` (workspace), `rust-toolchain.toml`, `README.md`

**Step 1:** `git init` in `/home/guy/projects/gameulator`.

**Step 2:** Write `.gitignore`:
```gitignore
/target
/games/            # ROMs, saves, snapshots — never tracked
**/*.gbc
**/*.sav
**/*.srm
/vendor/pokemon-yellow-legacy/   # submodule blobs; pointer tracked separately
```

**Step 3:** Write the workspace `Cargo.toml`:
```toml
[workspace]
resolver = "2"
members = ["crates/pokegen1", "crates/app", "crates/cli"]

[workspace.package]
edition = "2021"
license = "MIT"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
anyhow = "1"
thiserror = "2"
```

**Step 4:** Write `rust-toolchain.toml` pinning `channel = "stable"`.

**Step 5:** Create empty crate skeletons: `crates/pokegen1` (lib), `crates/app` (lib),
`crates/cli` (bin) each with a minimal `Cargo.toml` + `src/lib.rs`/`src/main.rs`.

**Step 6:** Run `cargo build`. Expected: workspace builds (empty crates).

**Step 7:** Move the two design docs are already in `docs/plans/`; `git add -A`.

**Step 8: Commit**
```bash
git add -A
git commit -m "chore: bootstrap gameulator Rust workspace + design docs"
```

---

## Task 2: Add the pinned Yellow Legacy submodule & build the ROM

**Files:** Modify: `.gitmodules` (created by submodule add)

**Step 1:** Add the submodule:
```bash
git submodule add https://github.com/cRz-Shadows/Pokemon_Yellow_Legacy vendor/pokemon-yellow-legacy
```

**Step 2:** Pin to the v1.0.10 release tag (verify exact tag name first):
```bash
cd vendor/pokemon-yellow-legacy
git tag | grep -i 1.0.10        # find exact tag spelling
git checkout <exact-v1.0.10-tag>
cd -
```

**Step 3:** Ensure **RGBDS exactly 0.6.1** is available (newer versions break the build).
Check `rgbasm --version`; if wrong/missing, build 0.6.1 from source per the repo's
INSTALL.md. **Log the install** in `/usr/share/CHANGELOG.md` per machine policy.

**Step 4:** Build the ROM:
```bash
make -C vendor/pokemon-yellow-legacy
```
Expected: produces `vendor/pokemon-yellow-legacy/pokeyellow.gbc`.

**Step 5:** Copy the ROM into the gitignored game dir:
```bash
mkdir -p "games/Pokemon/Yellow Legacy/rom"
cp vendor/pokemon-yellow-legacy/pokeyellow.gbc "games/Pokemon/Yellow Legacy/rom/"
```

**Step 6:** Add a `Makefile` (or `xtask` cmd) target `rom` wrapping steps 4–5 so the
build is one command (locality: one place to change the build).

**Step 7: Commit**
```bash
git add .gitmodules vendor/pokemon-yellow-legacy Makefile
git commit -m "build: pin Yellow Legacy v1.0.10 submodule + ROM build target"
```

---

## Task 3: `pokegen1` — Gen-1 string decode (`gbstr`)

**Files:**
- Create: `crates/pokegen1/src/core/text.rs`, wire `core` module in `src/lib.rs`
- Test: same file (`#[cfg(test)]`)

**Step 1: Write failing test** — decode a known Gen-1 encoded name. `0x80..` = `A..Z`,
`0x50` terminates; `RED` = `[0x92,0x87,0x80,0x94,0x8D,0x50]`.
```rust
#[test]
fn decodes_name_until_terminator() {
    assert_eq!(decode_string(&[0x92,0x87,0x80,0x94,0x8D,0x50,0xFF]), "RED");
}
```

**Step 2:** Run `cargo test -p pokegen1 text` → FAIL (undefined).

**Step 3:** Implement `decode_string(bytes: &[u8]) -> String` with the full charmap from
`reference/read_save.py` (`CH`: letters, digits, punctuation, space `0x7F`), stopping at
`0x50`/`0x00`. Keep the charmap as a single `const`/table (one edit point).

**Step 4:** Run test → PASS. Add cases: space, punctuation, empty → "".

**Step 5: Commit** `feat(pokegen1): Gen-1 text decode (gbstr)`.

---

## Task 4: `pokegen1` — SRAM offset map + `SaveData` newtype

**Files:** Create `crates/pokegen1/src/core/sram.rs`

**Step 1: Test** — a `SaveData` wraps a byte buffer and exposes typed reads: `u8`,
big-endian `u16`, and a slice at an offset. Assert `read_u16_be` on `[0x12,0x34]` = 0x1234.

**Step 2:** Run → FAIL.

**Step 3:** Implement `SaveData(Vec<u8>)` + `read_u8`, `read_u16_be`, `slice`. Define an
**offsets module** as named consts (design §8): `NAME=0x2598`, `BAG=0x25CA`,
`PC_ITEMS=0x2834`, `PLAYTIME_H=0x2CED`, `PLAYTIME_M=0x2CEF`, `PARTY_COUNT=0x2F2C`,
`PARTY_DATA=0x2F34`, `OT_NAMES=0x2F9C`, `NICKNAMES=0x307E`, `PARTY_STRUCT_LEN=44`.
All offsets in **one** file (locality).

**Step 4:** Run → PASS.

**Step 5: Commit** `feat(pokegen1): SRAM accessor + offset map`.

---

## Task 5: `pokegen1` — party count + species list

**Files:** Create `crates/pokegen1/src/core/party.rs`; test helper `test_support::sram()`
that builds a synthetic buffer.

**Step 1: Test** — synthetic SRAM with `PARTY_COUNT`=2 and two species bytes; assert
`party_count()` == 2 and rejects counts <1 or >6 (returns `Err`/`None`).

**Step 2:** Run → FAIL.

**Step 3:** Implement `party_count(&SaveData) -> Result<u8>` (validate 1..=6).

**Step 4:** Run → PASS.

**Step 5: Commit** `feat(pokegen1): party count read + validation`.

---

## Task 6: `pokegen1` — parse one party `Pokemon` struct

**Files:** `crates/pokegen1/src/core/pokemon.rs`

**Step 1: Test** — build a synthetic 44-byte party struct at `PARTY_DATA` with known
species, **level at offset 0x21** (design §8 gotcha — NOT 0x03), current HP (BE u16 at
+0x01), max HP/atk/def/spd/spc (BE u16 at +0x22,+0x24,+0x26,+0x28,+0x2A), 4 move ids at
+0x08, PP+ppups at +0x1D (low 6 bits PP, top 2 bits PP-ups), status byte at +0x04.
Assert every field parses to the seeded values.

**Step 2:** Run → FAIL.

**Step 3:** Implement `struct Pokemon { species_id, level, hp, max_hp, atk, def, spd,
spc, moves: [MoveSlot;4], status }` + `parse_pokemon(&SaveData, slot) -> Pokemon`.
Port status-bit decode (SLEEP mask 0x07, POISON 0x08, BURN 0x10, FREEZE 0x20, PARALYZE
0x40; hp==0 ⇒ FAINTED) and PP decode from `read_save.py`.

**Step 4:** Run → PASS.

**Step 5: Commit** `feat(pokegen1): parse party Pokemon struct`.

---

## Task 7: `pokegen1` — full party (with nicknames)

**Step 1: Test** — 2-mon synthetic party incl. nicknames at `NICKNAMES` (11 bytes each);
assert `parse_party()` returns 2 `Pokemon` with names + nicks; nick suppressed when it
equals the species name.

**Steps 2–4:** Implement `parse_party(&SaveData) -> Vec<Pokemon>` looping `party_count`.

**Step 5: Commit** `feat(pokegen1): full party parse with nicknames`.

---

## Task 8: `pokegen1` — item lists (bag + PC)

**Files:** `crates/pokegen1/src/core/items.rs`

**Step 1: Test** — synthetic item list = (item,qty) pairs terminated by `0xFF`; bag at
`BAG`, PC at `PC_ITEMS+1` (PC has a leading count byte — port that offset detail).
Assert both parse to the seeded pairs.

**Steps 2–4:** Implement `read_item_list(&SaveData, start) -> Vec<(u8,u8)>` (0xFF-terminated).

**Step 5: Commit** `feat(pokegen1): bag + PC item list parse`.

---

## Task 9: `pokegen1` — trainer name, playtime, SRAM checksum

**Step 1: Test** — synthetic name/playtime; assert `trainer_name()` and
`playtime()`=(h,m). Then seed a buffer, compute the Gen-1 SRAM **checksum** (sum of the
main-data range, complemented — verify exact algorithm against the disassembly's
`SAVEChecksum`), and assert `verify_checksum()` is true; flip a byte → false.

**Steps 2–4:** Implement `trainer_name`, `playtime`, and `verify_checksum` (the checksum
underpins the sync watcher's corruption guard).

**Step 5: Commit** `feat(pokegen1): trainer/playtime/checksum`.

---

## Task 10: `pokegen1` — top-level `Save` + `parse_save`

**Files:** `crates/pokegen1/src/core/save.rs`, re-export from `lib.rs`

**Step 1: Test** — assemble a synthetic full save; assert `parse_save(&[u8]) -> Save`
yields the combined `{ trainer, playtime, party, bag, pc, checksum_ok }`.

**Steps 2–4:** Implement `struct Save {…}` + `parse_save`. This is the public Model
entry point.

**Step 5:** Integration test: read `games/Pokemon/Yellow Legacy/saves/*.sav` if present,
else skip. **Commit** `feat(pokegen1): top-level Save parse`.

---

## Task 11: Traits for variant data (species/move/type tables)

**Files:** `crates/pokegen1/src/core/data.rs`

**Step 1: Test** — define trait objects `SpeciesTable`, `MoveTable`, `TypeChart`; a stub
impl returns a name for an id; assert lookups. This is the seam the overlay fills.

**Steps 2–4:** Define the traits (`fn species_name(&self, id:u8)->Option<&str>`, etc.).
Consumers depend on these, never on concrete tables (locality/flexibility).

**Step 5: Commit** `feat(pokegen1): variant data traits (species/move/type)`.

---

## Task 12: `xtask` data generator from the disassembly

**Files:** Create `crates/xtask` (bin), add to workspace; output
`crates/pokegen1/src/games/yellow_legacy/generated/*.json`

**Step 1: Test** — a small parser fn over a fixture `.asm` snippet (e.g. one
`base_stats` entry, one `moves.asm` line, a `type_matchups.asm` triple) returns the
expected structs. Port the regexes from `reference/parse_legacy.py`.

**Steps 2–4:** Implement generators reading `vendor/pokemon-yellow-legacy/{data,constants}`
→ emit JSON tables (species base stats+types, moves, type chart incl. Legacy's Ghost=
special, TM/HM lists, evos/learnsets, trainer/boss parties). One `xtask gen-data` command.
Note: regenerating is the ONLY way tables change (never hand-edit) — put a header comment
saying so in each generated file.

**Step 5: Commit** `feat(xtask): generate Yellow Legacy data tables from disassembly`.

---

## Task 13: `yellow_legacy` overlay — implement the traits over generated data

**Files:** `crates/pokegen1/src/games/yellow_legacy/mod.rs`

**Step 1: Test** — load the generated JSON; assert `YellowLegacy` impls `SpeciesTable`/
`MoveTable`/`TypeChart` (e.g. species 131 = MEWTWO; Ghost→Psychic super-effective per
Legacy). Assert Legacy override facts: bag capacity 41; Pikachu badge boost ×1.25.

**Steps 2–4:** Implement the overlay: embed generated JSON (`include_str!`), impl the
traits, encode Legacy constants (Ghost special, Pikachu ×1.25, bag 41, `wDifficulty`
awareness as a `Difficulty` enum surfaced on `Save`).

**Step 5: Commit** `feat(pokegen1): Yellow Legacy overlay over generated tables`.

---

## Task 14: `app` controller — presentation-agnostic ops

**Files:** `crates/app/src/lib.rs`

**Step 1: Test** — `load_save(path, game) -> SaveView`; `party_summary(&SaveView) ->
Vec<PartyLine>` (species name resolved via the trait, effective stats, moves w/ PP);
assert against a synthetic save + stub game. All logic lives here **once**.

**Steps 2–4:** Implement `load_save`, `party_summary`, and stubs for `diff`,
`type_coverage`, `dex_progress` returning plain structs (serde-derive for `--json`).

**Step 5: Commit** `feat(app): controller ops (load/party_summary/…)`.

---

## Task 15: `cli` — render parity with the Termux scripts

**Files:** `crates/cli/src/main.rs`

**Step 1: Test** — an integration test invoking the built binary (`assert_cmd`) on a
synthetic save file prints the trainer + party lines; `--json` emits valid JSON that
round-trips.

**Steps 2–4:** Implement `clap` commands: `party [--json|--compact]`, `bag`, `pc`,
`info` (trainer/playtime/badges). Reuse `app` ops; **views only format** (locality:
formatting here, logic in `app`). Match the readable output of `read_save.py`.

**Step 5:** Run against the real synced save (if present) and eyeball vs the old Python.
**Commit** `feat(cli): party/bag/pc/info views with --json`.

---

## Task 16: Milestone-1 wrap

**Step 1:** `cargo test` (all crates) green; `cargo clippy` clean.
**Step 2:** Update `README.md`: build, `xtask gen-data`, CLI usage.
**Step 3:** Move `reference/` scripts' unique lessons already captured in design §8;
leave scripts as reference.
**Step 4: Commit** `docs: Milestone 1 usage + wrap`.

---

## Milestone 2 — Sync (separate detailed plan)

Summary only; gets its own `docs/plans/` file once M1 lands and open items resolve:
- `crates/sync`: inotify watcher on the synced `.sav`; on change → `verify_checksum` →
  snapshot to `saves/snapshots/<ts>.sav` → **monotonic-playtime regression alarm** →
  emit a change event.
- Syncthing setup: phone (Android app) + computer daemon sharing `saves/`; document the
  active-device marker convention.
- Tests: feed the watcher old→new and new→old saves; assert snapshot + regression flag.

## Milestone 3 — Web (separate detailed plan)

Summary only:
- `crates/web`: Leptos (WASM) frontend calling the **same `app` ops** compiled to WASM;
  axum host serving assets + the live `.sav` bytes; WebSocket fed by M2's change events.
- Views: dex grid, type-coverage heatmap, party/box cards, run timeline.
- Decide: data tables as JSON vs RON vs build.rs; axum↔WASM save-bytes endpoint shape.

---

## Notes for the executor

- **Do not hand-edit generated data tables** — change the `xtask` generator (locality).
- **Trust struct offset 0x21 for party level**, not 0x03 (design §8).
- If a parsed value disagrees with a known-good value from the real save, use
  @superpowers:systematic-debugging — the offset map is the usual suspect.
- Keep formatting in `cli`, logic in `app`, parsing in `pokegen1` — the layer boundary
  IS the locality guarantee.
