# Gameulator Milestone 2 — Save Sync (Design)

> Status: **validated in brainstorming (2026-07-17), ready for an implementation plan**
> Companion: `docs/plans/2026-07-17-gameulator-design.md` §4, §6.

**Goal:** Seamless phone↔computer swap by syncing the Yellow Legacy `.sav`, with a
watcher that validates, snapshots (automatic diff history), and guards against
clobbering a newer save with a stale one.

**Sequencing decision:** **watcher crate first** (testable in isolation via synthetic
file events, keeps M2 in the code/SDD groove), then wire Syncthing underneath. The
Syncthing setup is a short manual step (user's devices), documented as a runbook, not
code.

## Transport — Syncthing (manual setup, documented)
Phone runs the Syncthing **Android app** (F-Droid — reliable background sync); computer
runs the daemon; they share the `ROMs/saves/` folder ↔
`games/Pokemon/Yellow Legacy/saves/`. LAN-direct, bidirectional, encrypted, no cloud.
This lives in a README/runbook (`docs/` or repo README), NOT the crate.

**Granularity:** Pizza Boy writes the `.sav` only on an in-game SAVE (battery save);
save-states are a separate format we deliberately do NOT sync. So the sync unit is
"per in-game save" — the correct handoff unit.

## The `sync` crate (the code deliverable)
A watcher daemon layering intelligence over Syncthing's raw file mirror:

1. **Watch** `games/Pokemon/Yellow Legacy/saves/*.sav` via the `notify` crate
   (inotify), **debounced** — Syncthing writes in bursts; coalesce rapid events and
   act on the settled file.
2. **Validate** on change: `pokegen1::core::checksum::verify_checksum`. A mid-write or
   corrupt file fails → **quarantine** (log loudly, skip; the next settled event picks
   up the completed write). This is why `verify_checksum` was built in M1.
3. **Snapshot** valid saves → `saves/snapshots/<ISO-8601-timestamp>.sav`. Retention:
   **keep all** (32 KB each; thousands ≈ a few MB). Add pruning only if it ever matters.
   This is the automatic successor to `reference/pokedump.sh`'s manual dump history.
4. **Regression alarm (the soft "lock")**: parse playtime (+ badges/party) via
   `pokegen1`; if an incoming save is *behind* the latest snapshot (less playtime) →
   **loud alarm**, preserve the newer snapshot, do NOT let the stale one become the
   "current" pointer. This replaces a fragile hard device-lock (matches the user's
   honor-system stance). **No explicit active-device marker in v1** (YAGNI) — the
   regression check + Syncthing's own `.sync-conflict` files cover it.
5. **Emit**: write `saves/status.json` — a parsed summary of the current save
   (trainer, party, playtime, checksum_ok, last-change timestamp) — plus a log line.
   This file is the **M3 seam**: the web dashboard's WebSocket reads/subscribes to it
   later. Optionally fire a desktop/phone notification (ntfy) on change.

## Testing (watcher-first ⇒ no Syncthing needed to test)
- Feed the watcher synthetic file changes in a temp dir; assert: corrupt file →
  quarantined (no snapshot); valid file → snapshot written + `status.json` updated;
  old→new playtime → accepted; **new→old playtime → regression alarm, newer snapshot
  preserved**.
- Debounce: rapid burst of writes → one settled action.

## Open items (resolve in the implementation plan)
- Exact `status.json` schema (likely reuse `app`'s `SaveInfoView` + a party summary).
- Config: save path / snapshot dir (probably a small config, aligning with the
  future ROM-source config from the roadmap's data-sourcing section).
- Whether the watcher also emits over a channel now (for M3) or only writes
  `status.json` (simplest — M3 adds the WebSocket).

## Deferred to later milestones (not M2)
- WebSocket push + the web dashboard itself (M3).
- Reading `wDifficulty` from the save (needs a disassembly offset-hunt) — surface it in
  `status.json` once located.
