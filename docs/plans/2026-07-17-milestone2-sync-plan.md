# Gameulator Milestone 2 — Sync Watcher Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (or
> superpowers:subagent-driven-development) to implement this plan task-by-task.
> Design: `docs/plans/2026-07-17-milestone2-sync-design.md`.

**Goal:** A Rust `sync` watcher crate that watches the synced Yellow Legacy `.sav`,
validates it (checksum), snapshots valid saves with a timestamp, alarms on a
playtime regression (stale-device clobber guard), and writes a `status.json`
summary — all testable without Syncthing.

**Architecture:** The watcher's *logic* is factored into pure/injectable functions
(`process_save`, regression comparison, snapshot naming with an injected timestamp)
so it's unit- and tempdir-testable. A thin `notify`-based debounced event loop calls
`process_save` on settled file changes. Reuses `pokegen1` (parse + `verify_checksum`)
and `app` (view DTOs for `status.json`). Syncthing is a documented manual setup, not
code.

**Tech Stack:** Rust, `notify` + `notify-debouncer-full` (file watching), `chrono`
(snapshot timestamps — injected into pure fns so tests stay deterministic),
`serde`/`serde_json` (`status.json`), `anyhow`, `tempfile` (dev).

**Governing principle (user):** "one conceptual change = one contiguous edit block."
Keep the watcher loop thin; put logic in testable functions.

---

## Conventions
- **TDD always.** Failing test → red → minimal impl → green → commit.
- **Inject the clock.** Snapshot naming + `status.json` timestamps take a
  `chrono::DateTime` (or a preformatted string) as a PARAMETER, so tests pass a fixed
  value. Only the thin watcher/`main` calls `Utc::now()`.
- **Tests use `tempfile::tempdir()`** for the save/snapshots/status paths via `Config`.
- Commit after every green step. Run `cargo test -p sync`, `cargo clippy -p sync
  --all-targets`, keep `cargo test --workspace` green.
- Reuse existing pokegen1 APIs: `pokegen1::parse_save(Vec<u8>) -> Result<Save,_>`,
  `pokegen1::core::checksum::verify_checksum(&SaveData)` /
  `pokegen1::core::sram::SaveData`. NOTE: `verify_checksum` takes a `SaveData`;
  `parse_save` also reports `Save.checksum_ok`. Prefer `Save.checksum_ok` where a full
  parse is already needed; use `verify_checksum` directly for a cheap validity gate.

---

## Task 1: Scaffold the `sync` crate + `Config`

**Files:**
- Create: `crates/sync/Cargo.toml`, `crates/sync/src/lib.rs`, `crates/sync/src/main.rs`
- Modify: root `Cargo.toml` (add `crates/sync` to `members`), `[workspace.dependencies]`
  (add `notify = "6"`, `notify-debouncer-full = "0.3"`, `chrono = { version = "0.4",
  default-features = false, features = ["clock"] }`)

**Step 1:** Add `crates/sync` to workspace `members`. Add the three deps to
`[workspace.dependencies]`.

**Step 2:** `crates/sync/Cargo.toml`: a lib + bin crate named `sync`, bin name
`gameulator-sync`. Deps: `pokegen1` (path), `app` (path), `notify`,
`notify-debouncer-full`, `chrono`, `serde`, `serde_json`, `anyhow` (workspace where
applicable). Dev-deps: `tempfile = "3"`, `pokegen1` (path, for building save fixtures).

**Step 3:** `src/lib.rs`: crate doc + a `Config`:
```rust
use std::path::PathBuf;
use std::time::Duration;

/// Paths + tuning the watcher operates on. Tests point these at tempdirs.
#[derive(Debug, Clone)]
pub struct Config {
    /// The synced `.sav` file to watch.
    pub save_path: PathBuf,
    /// Directory where timestamped snapshots are written.
    pub snapshots_dir: PathBuf,
    /// Where the parsed-summary `status.json` is written.
    pub status_path: PathBuf,
    /// Debounce window for coalescing Syncthing's burst writes.
    pub debounce: Duration,
}
```
Add `Config::for_game_dir(saves_dir: &Path) -> Config` deriving the standard layout
(`save_path` = the single `*.sav` in `saves_dir` or a documented default;
`snapshots_dir = saves_dir/snapshots`; `status_path = saves_dir/status.json`;
`debounce = Duration::from_secs(2)`).

**Step 4:** `src/main.rs`: `fn main() -> anyhow::Result<()>` stub that builds a
`Config` and prints it (real wiring in Task 9).

**Step 5:** `cargo build -p sync` succeeds. **Commit:** `chore(sync): scaffold sync crate + Config`.

---

## Task 2: Checksum-validation gate

**Files:** Create `crates/sync/src/validate.rs`; `pub mod validate;` in lib.rs.

**Step 1: Test** (use a helper that builds a valid full save with a correct checksum —
port the pattern from `crates/pokegen1/src/core/save.rs` tests, or build via
`pokegen1::core::sram` consts + compute the checksum). Assert:
- `is_valid_save(&good_bytes)` == `true`
- flipping an in-range byte (without fixing the checksum) → `false`
- a too-short buffer (e.g. 100 bytes) → `false` (no panic).

**Step 2:** Run → FAIL.

**Step 3:** Implement `pub fn is_valid_save(bytes: &[u8]) -> bool` = `parse_save`
succeeds AND `save.checksum_ok`. (Truncated/invalid-count → `parse_save` `Err` → not
valid; bad checksum → `Ok` but `checksum_ok == false` → not valid.) This is the
quarantine gate: only fully-valid saves proceed.

**Step 4:** Run → PASS. **Commit:** `feat(sync): checksum validation gate`.

---

## Task 3: Snapshot writing (injected timestamp)

**Files:** Create `crates/sync/src/snapshot.rs`; `pub mod snapshot;`.

**Step 1: Test** (tempdir): `write_snapshot(&snapshots_dir, &bytes, "2026-07-17T14-30-00Z")`
creates `snapshots_dir/2026-07-17T14-30-00Z.sav` with exactly `bytes`; creates the dir
if missing; returns the written `PathBuf`.

**Step 2:** Run → FAIL.

**Step 3:** Implement `pub fn write_snapshot(dir: &Path, bytes: &[u8], stamp: &str) ->
anyhow::Result<PathBuf>` — `create_dir_all(dir)`, write `dir/{stamp}.sav`, return path.
Also `pub fn stamp_now() -> String` using `chrono::Utc::now()` formatted
filesystem-safe (e.g. `%Y-%m-%dT%H-%M-%S%.3fZ` — no colons; millis avoid same-second
collisions). `stamp_now` is only called by the watcher; `write_snapshot` takes the
stamp as a param so tests are deterministic.

**Step 4:** Run → PASS. **Commit:** `feat(sync): timestamped snapshot writer`.

---

## Task 4: Latest-snapshot lookup + playtime

**Files:** Add to `snapshot.rs`.

**Step 1: Test** (tempdir): with snapshots `A.sav`(playtime 10h0m) and `B.sav`(20h0m)
written (ISO names sort chronologically), `latest_snapshot_playtime(&dir)` returns
`Some(20*60 + 0)` total minutes (the newest by filename sort). Empty dir → `None`.

**Step 2:** Run → FAIL.

**Step 3:** Implement `pub fn latest_snapshot_playtime(dir: &Path) ->
anyhow::Result<Option<u32>>`: list `*.sav` in `dir`, pick the lexicographically-last
name (ISO stamps sort chronologically), `parse_save` it, return
`Some(playtime.hours as u32 * 60 + playtime.minutes as u32)`. Missing dir / no
snapshots → `Ok(None)`. (A corrupt snapshot → skip it or `Err`; keep simple: parse the
newest, propagate error — snapshots we wrote are always valid.)

**Step 4:** Run → PASS. **Commit:** `feat(sync): latest-snapshot playtime lookup`.

---

## Task 5: Regression comparison (pure)

**Files:** Create `crates/sync/src/regression.rs`; `pub mod regression;`.

**Step 1: Test** — pure fn, no IO:
- `check(incoming=1200, latest=Some(1000))` → `Accept` (progressed)
- `check(1000, Some(1000))` → `Accept` (equal is fine — a re-save)
- `check(900, Some(1000))` → `Regression { incoming: 900, latest: 1000 }` (stale!)
- `check(500, None)` → `Accept` (first snapshot, nothing to compare)

**Step 2:** Run → FAIL.

**Step 3:** Implement:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegressionCheck {
    Accept,
    Regression { incoming: u32, latest: u32 }, // incoming playtime < latest
}
/// Playtime is monotonic in normal play, so an incoming save with LESS total
/// playtime than the latest snapshot means a stale device overwrote a newer save.
pub fn check(incoming_minutes: u32, latest_minutes: Option<u32>) -> RegressionCheck {
    match latest_minutes {
        Some(latest) if incoming_minutes < latest => {
            RegressionCheck::Regression { incoming: incoming_minutes, latest }
        }
        _ => RegressionCheck::Accept,
    }
}
```

**Step 4:** Run → PASS. **Commit:** `feat(sync): playtime regression check`.

---

## Task 6: `status.json` writer

**Files:** Create `crates/sync/src/status.rs`; `pub mod status;`.

**Step 1: Test** (tempdir): given a parsed `Save`, `write_status(&status_path, &save,
&game, "2026-07-17T14-30-00Z", Some(snapshot_path))` writes JSON that deserializes and
contains the trainer name, playtime, `checksum_ok`, a non-empty `party` summary, and
the `last_change` stamp. (`game` = `app::game_data(app::GameId::YellowLegacy)`.)

**Step 2:** Run → FAIL.

**Step 3:** Define `StatusView { trainer, playtime, checksum_ok, party:
Vec<app::PartyMemberView>, last_change: String, snapshot: Option<String> }` (serde
Serialize). Build it from `app::save_info(save)` + `app::party_summary(save,
game.as_ref())`. `pub fn write_status(path, save, game, stamp, snapshot) ->
anyhow::Result<()>` serializes pretty + writes. This file is the **M3 web seam**.

**Step 4:** Run → PASS. **Commit:** `feat(sync): status.json writer (M3 seam)`.

---

## Task 7: `process_save` pipeline (the heart)

**Files:** Create `crates/sync/src/process.rs`; `pub mod process;`.

**Step 1: Test** (tempdir `Config`) — the key behaviors, each its own test:
- **corrupt → quarantine:** feed invalid bytes → returns `Outcome::Quarantined`; NO
  snapshot written; `status.json` NOT updated.
- **valid → snapshot + status:** feed a valid save (playtime 20h) → `Outcome::Applied {
    regression: None, .. }`; a `.sav` appears in `snapshots_dir`; `status.json` exists
  with trainer/party.
- **new→old → regression alarm:** first apply a valid 20h save, then feed a valid 10h
  save → `Outcome::Applied { regression: Some(RegressionCheck::Regression{..}), .. }`;
  the 10h snapshot IS still written (we keep all), but the outcome flags the regression
  so the watcher logs loudly. (Design: preserve the newer snapshot; the alarm is the
  guard, not deletion.)

**Step 2:** Run → FAIL.

**Step 3:** Implement:
```rust
pub enum Outcome {
    Quarantined { reason: String },
    Applied { snapshot: PathBuf, regression: RegressionCheck },
}
/// Validate → snapshot → regression-check → write status. `now` is injected.
pub fn process_save(cfg: &Config, game: &dyn app::GameData, bytes: &[u8], stamp: &str)
    -> anyhow::Result<Outcome>
```
Flow: `validate::is_valid_save(bytes)` false → `Ok(Quarantined{..})` (log reason).
Else: `parse_save` (already valid), compute `incoming_minutes`;
`latest = snapshot::latest_snapshot_playtime(&cfg.snapshots_dir)?`;
`reg = regression::check(incoming_minutes, latest)`; `snap =
snapshot::write_snapshot(&cfg.snapshots_dir, bytes, stamp)?`;
`status::write_status(&cfg.status_path, &save, game, stamp, Some(snap.clone()))?`;
`Ok(Applied { snapshot: snap, regression: reg })`.

**Step 4:** Run → PASS. **Commit:** `feat(sync): process_save validate/snapshot/regression/status pipeline`.

---

## Task 8: Debounced `notify` watcher

**Files:** Create `crates/sync/src/watch.rs`; `pub mod watch;`.

**Step 1: Test** (integration, tempdir, short debounce e.g. 200ms): start the watcher
on a temp `save_path` in a background thread; write a valid save to that path; poll for
up to ~5s until `status.json` + a snapshot appear; assert they do. (Use a channel or
poll-with-timeout; keep it robust against timing.)

**Step 2:** Run → FAIL.

**Step 3:** Implement `pub fn run(cfg: Config, game: Box<dyn app::GameData>) ->
anyhow::Result<()>` (blocking): set up `notify-debouncer-full` watching
`cfg.save_path`'s parent (non-recursive), filter events for `cfg.save_path`; on a
debounced event, `std::fs::read(&cfg.save_path)` then
`process::process_save(&cfg, game.as_ref(), &bytes, &snapshot::stamp_now())`; **log**
the outcome (info on Applied, **warn loudly** on a `Regression`, warn on Quarantined).
Keep the loop thin — all logic is in `process_save`. Use `eprintln!` for logging (or a
tiny logger; avoid heavy deps — YAGNI).

**Step 4:** Run → PASS (may be slower; note the timing). **Commit:** `feat(sync): debounced notify watcher loop`.

---

## Task 9: `main.rs` wiring + CLI

**Files:** Modify `crates/sync/src/main.rs`.

**Step 1: Test** (assert_cmd, dev-dep): `gameulator-sync --help` shows usage with a
`--saves-dir` option (or a positional). (Add `clap` + `assert_cmd` dev/deps as needed.)

**Step 2–3:** `main`: parse args (`--saves-dir <dir>`, default `games/Pokemon/Yellow
Legacy/saves`), build `Config::for_game_dir`, `let game =
app::game_data(app::GameId::YellowLegacy);`, print a startup line (watching path,
snapshots dir), then `watch::run(cfg, game)`. On startup, optionally process the
current save once so `status.json` is fresh immediately.

**Step 4:** Run help → PASS. **Commit:** `feat(sync): gameulator-sync binary + CLI`.

---

## Task 10: Syncthing runbook + Milestone-2 wrap

**Files:** Create `docs/SYNCTHING.md`; modify `README.md`.

**Step 1:** `docs/SYNCTHING.md` — the manual setup runbook: install Syncthing Android
app (F-Droid) + the computer daemon; share the phone `ROMs/saves/` folder with
`games/Pokemon/Yellow Legacy/saves/`; verify a save on the phone appears on the
computer; then run `cargo run -p sync -- --saves-dir "games/Pokemon/Yellow Legacy/saves"`
(or the installed `gameulator-sync`). Document the `snapshots/` + `status.json` outputs
and the regression-alarm behavior.

**Step 2:** README: add a "Sync (Milestone 2)" section pointing at the runbook + the
`gameulator-sync` binary; update the §Status deferrals (sync now partial: watcher done,
Syncthing = manual setup).

**Step 3:** Verify: `cargo test --workspace` green; `cargo clippy --workspace
--all-targets` clean; `cargo fmt --all --check` clean.

**Step 4:** **Commit:** `docs: Syncthing runbook + M2 wrap`.

---

## Notes for the executor
- **Keep the watcher loop thin;** all decision logic lives in `process_save` and the
  pure helpers — that's what makes M2 testable without Syncthing.
- **Inject timestamps** into `write_snapshot`/`write_status`; only `stamp_now()` and the
  watcher call the real clock.
- **Regression = alarm, not deletion.** Always keep the snapshot; the `Regression`
  outcome makes the watcher log loudly. Never silently drop or overwrite.
- **`status.json` is the M3 seam** — keep it a clean serialization of the `app` DTOs +
  change metadata.
- Deferred (not M2): WebSocket push (M3), `wDifficulty` in `status.json` (needs the
  offset-hunt), an explicit active-device marker (regression check suffices for v1).
