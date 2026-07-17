# Syncthing Save-Sync Runbook (Milestone 2)

Manual setup for mirroring your Pokémon Yellow Legacy battery save between phone
and computer, plus running the `gameulator-sync` watcher over the mirrored file.

## What & why

Syncthing mirrors the phone's Pizza Boy `.sav` to the computer over your LAN —
**no cloud, LAN-direct, encrypted, bidirectional**. Once the file lands on the
computer, the `gameulator-sync` watcher takes over: it validates the save
(checksum), snapshots every valid write (keep-all history), guards against a
stale device clobbering a newer save (a playtime **regression alarm**), and
writes a parsed `status.json` summary for the future web view.

**The sync unit is one in-game SAVE.** Pizza Boy writes the battery `.sav` only
when you SAVE in-game (talk to your mom / use a PC / the in-game save menu). This
is the correct handoff unit for swapping devices. **Save-states are a separate
format and are deliberately NOT synced** — only the battery `.sav`.

## Phone setup

1. Install the **Syncthing Android app from F-Droid**
   (<https://f-droid.org/packages/com.nutomic.syncthingandroid/>).
   > Prefer the F-Droid app over a Termux-built daemon — it has far more reliable
   > background sync (foreground service + battery-optimization handling) than
   > Termux, which the OS tends to kill.
2. Open the app and let it generate the device ID.
3. Add a folder that points at Pizza Boy's saves directory — typically
   `ROMs/saves/` under wherever Pizza Boy stores its files. Give it a stable
   folder ID (e.g. `pizzaboy-saves`).
4. Share that folder with your computer's Syncthing device (you'll accept it on
   the computer side below).
   > You may need to do the computer setup first so its device ID exists to
   > share with. The folder **ID** (`pizzaboy-saves`) is what's shared and must
   > match on both devices; the local **path** differs per device (see below).

## Computer setup

1. Install and run the Syncthing daemon:
   ```sh
   # Debian/Ubuntu
   sudo apt install syncthing
   systemctl --user enable --now syncthing.service
   # or just run it in a terminal:
   syncthing
   ```
   The web UI is at <http://127.0.0.1:8384>.
2. Pair the phone and computer devices (scan the QR / exchange device IDs).
   **Accept the incoming _device_ first** — the shared-folder offer only appears
   once the two devices are connected (this device-then-folder order is the most
   common first-time snag).
3. When the phone then offers the shared `ROMs/saves/` folder, **accept it and
   set its local path to the gitignored saves dir**:
   ```
   games/Pokemon/Yellow Legacy/saves/
   ```
   (relative to the repo root — this is the same directory the watcher defaults
   to). Create it first if it doesn't exist.

## Verify the mirror

1. On the phone, load Yellow Legacy in Pizza Boy and do an **in-game SAVE**.
2. Wait a few seconds for Syncthing to sync.
3. On the computer, confirm the updated `.sav` appears in
   `games/Pokemon/Yellow Legacy/saves/`:
   ```sh
   ls -l "games/Pokemon/Yellow Legacy/saves/"
   ```
   The `.sav`'s modification time should match your in-game save. If it does, the
   transport is working — you can now run the watcher.

## Run the watcher

From the **repo root** (see the CWD note below):

```sh
cargo run -p sync --bin gameulator-sync -- --saves-dir "games/Pokemon/Yellow Legacy/saves"
```

Or, if you've installed the binary:

```sh
gameulator-sync --saves-dir "games/Pokemon/Yellow Legacy/saves"
```

`--saves-dir` defaults to `games/Pokemon/Yellow Legacy/saves`, so from the repo
root you can just run `cargo run -p sync --bin gameulator-sync`.

On startup it prints the three paths it operates on, then does a startup pass
(processing the current save immediately) and blocks forever (Ctrl-C to stop).

### Outputs

Both live inside the saves dir, alongside the `.sav`:

- **`snapshots/<timestamp>.sav`** — one snapshot per valid save, named with a
  filesystem-safe, chronologically-sortable UTC stamp
  (e.g. `2026-07-17T14-30-00.123Z.sav`). **Keep-all history** — nothing is
  pruned (each is ~32 KB; thousands total only a few MB). This is the automatic
  successor to manual save dumps.
- **`status.json`** — a parsed summary of the current save: trainer, playtime,
  `checksum_ok`, a resolved party summary, the `last_change` timestamp, and the
  `snapshot` path for the change. It is written atomically (temp file + rename),
  so a reader never sees a torn/partial file. **The future web view (Milestone 3)
  reads this file.**

## Operator notes

Read these so the watcher's output doesn't surprise you.

- **All watcher log output — including alarms — goes to stderr.** Redirect
  stderr if you want to capture it (`gameulator-sync ... 2> sync.log`).

- **The wait-state line is NORMAL, not an error.** If the save isn't present yet
  you'll see:
  ```
  [sync] no save at <path> yet — waiting for it to appear
  ```
  The watcher stays up and processes the save the moment it appears (e.g. after
  the first phone sync).

- **`⚠ REGRESSION` line.** This means an incoming save is *behind* the latest
  snapshot (less total playtime) — you likely played a **STALE device**. The
  newer snapshot is preserved; the stale save is **still snapshotted** (keep-all)
  and `status.json` still updates. It's an **alarm, not a deletion** — nothing is
  lost, but you should figure out which device is authoritative before continuing.

- **`⚠ N .sav files` line.** The saves dir contains more than one `.sav`. The
  watcher only watches the **lexicographically-first** one. Keep exactly one
  `.sav` in that directory to avoid confusion.

- **Coalesced / settled writes.** Syncthing writes in bursts. A rapid burst of
  writes within the debounce window (**default 2s**) is coalesced and processed
  **once**, on the settled bytes — you get one snapshot per logical save, not one
  per underlying write.

- **CWD dependency.** The default `--saves-dir` is a **relative path**, so either:
  - run the watcher from the **repo root**, OR
  - pass an **absolute** `--saves-dir`, OR
  - if daemonizing under systemd, set `WorkingDirectory=` (or use an absolute
    path — see the unit below).

  A wrong CWD isn't a crash — the watcher just parks in the wait-state on a
  non-existent path and never processes anything.

- **Two snapshots in the same startup second is expected.** If a Syncthing write
  coincides with launch, the startup pass plus the first watched event can each
  produce a snapshot within the same second. This is harmless.

## Optional: run it in the background (systemd user unit)

This unit runs the *installed* binary — first `cargo install --path crates/sync`
(puts `gameulator-sync` in `~/.cargo/bin`). It orders `After=syncthing.service`
but doesn't `Require` it: the watcher tolerates Syncthing being down (it just
parks in the wait-state), so don't add a hard `Requires=`.

`~/.config/systemd/user/gameulator-sync.service`:

```ini
[Unit]
Description=Gameulator save-sync watcher
After=syncthing.service

[Service]
# Absolute path avoids the relative-CWD gotcha above.
ExecStart=%h/.cargo/bin/gameulator-sync --saves-dir "%h/projects/gameulator/games/Pokemon/Yellow Legacy/saves"
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
```

Then:

```sh
systemctl --user daemon-reload
systemctl --user enable --now gameulator-sync.service
journalctl --user -u gameulator-sync -f   # watcher logs (stderr)
```
