//! The debounced `notify` watcher loop — the ONE non-pure, thin piece.
//!
//! `run` sets up a debounced filesystem watcher on the save's parent directory,
//! receives coalesced change batches, and delegates EVERY decision to
//! [`process::process_save`], then LOGS the structured [`Outcome`]. All real
//! logic lives in the pure helpers; this module only wires the watcher, filters
//! events for the save path, and logs. Errors (read failures mid-rename,
//! transient status.json write failures) are LOGGED-AND-CONTINUE — they must
//! never kill the long-running watcher.

use std::sync::mpsc;

use anyhow::Context;
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::new_debouncer;
use pokegen1::GameData;

use crate::regression::RegressionCheck;
use crate::{process, snapshot, Config};

/// Run the blocking, debounced watcher loop over `cfg.save_path`.
///
/// Watches the save's PARENT directory (non-recursive): Syncthing may replace
/// the file via rename/temp, so watching the containing dir is more robust than
/// watching a single inode. Every batch of debounced events that touches
/// `cfg.save_path` triggers exactly one [`handle_change`], which reads the
/// settled file and delegates to [`process::process_save`].
///
/// The `Send` bound on `game` lets callers move the resolver onto this thread
/// (e.g. the integration test spawns `run` on a `std::thread`). Blocks forever
/// (until the channel closes / the watcher is dropped).
pub fn run(cfg: Config, game: Box<dyn GameData + Send>) -> anyhow::Result<()> {
    let dir = cfg
        .save_path
        .parent()
        .with_context(|| {
            format!(
                "save_path has no parent directory: {}",
                cfg.save_path.display()
            )
        })?
        .to_path_buf();

    let (tx, rx) = mpsc::channel();
    let mut debouncer =
        new_debouncer(cfg.debounce, None, tx).context("creating filesystem debouncer")?;

    debouncer
        .watcher()
        .watch(&dir, RecursiveMode::NonRecursive)
        .with_context(|| format!("watching directory {}", dir.display()))?;

    // Blocking loop: the debouncer delivers coalesced batches on `rx`.
    for result in rx {
        match result {
            Ok(events) => {
                // Coalesce: handle ONCE per batch even if several events in the
                // batch touch the save.
                let touches_save = events
                    .iter()
                    .any(|ev| ev.event.paths.iter().any(|p| p == &cfg.save_path));
                if touches_save {
                    handle_change(&cfg, game.as_ref());
                }
            }
            // A watcher error is logged and the loop CONTINUES — a transient FS
            // hiccup must not tear down the watcher.
            Err(errors) => {
                for e in errors {
                    eprintln!("[sync] watch error: {e}");
                }
            }
        }
    }

    Ok(())
}

/// Read the settled save and run it through the pipeline, LOGGING the structured
/// outcome. Never panics or propagates: a read error (file mid-rename / not yet
/// present) and a pipeline error (e.g. a transient status.json write failure)
/// are both LOGGED-AND-RETURN so the watcher keeps running.
fn handle_change(cfg: &Config, game: &dyn GameData) {
    let bytes = match std::fs::read(&cfg.save_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[sync] could not read {}: {e}", cfg.save_path.display());
            return;
        }
    };

    match process::process_save(cfg, game, &bytes, &snapshot::stamp_now()) {
        Ok(process::Outcome::Applied {
            snapshot,
            regression,
        }) => match regression {
            RegressionCheck::Accept => {
                eprintln!("[sync] applied: {} (playtime OK)", snapshot.display());
            }
            RegressionCheck::Regression { incoming, latest } => {
                // The stale-device alarm. `latest - incoming` cannot underflow:
                // a Regression only exists when incoming < latest.
                eprintln!(
                    "[sync] ⚠ REGRESSION: incoming save is {} min behind the latest ({} < {}). \
You may have played a STALE device! The newer snapshot is preserved. \
This save was still snapshotted: {}",
                    latest - incoming,
                    incoming,
                    latest,
                    snapshot.display()
                );
            }
        },
        Ok(process::Outcome::Quarantined { reason }) => {
            eprintln!("[sync] quarantined (skipped): {reason}");
        }
        // A transient status.json write failure must NOT kill the watcher.
        Err(e) => {
            eprintln!("[sync] error processing save: {e:#}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use crate::{test_support, Config};

    #[test]
    fn watcher_processes_a_written_save_end_to_end() {
        let dir = tempfile::tempdir().unwrap();
        let save_path = dir.path().join("save.sav");
        let snapshots_dir = dir.path().join("snapshots");
        let status_path = dir.path().join("status.json");

        let cfg = Config {
            save_path: save_path.clone(),
            snapshots_dir: snapshots_dir.clone(),
            status_path: status_path.clone(),
            debounce: Duration::from_millis(200),
        };

        let game = app::game_data(app::GameId::YellowLegacy);
        {
            let cfg = cfg.clone();
            // Detach: `run` blocks forever. The thread leaks, but the test
            // process exits — acceptable for an integration test.
            std::thread::spawn(move || {
                let _ = crate::watch::run(cfg, game);
            });
        }

        // Give the watcher a moment to register with the OS before writing.
        std::thread::sleep(Duration::from_millis(100));
        std::fs::write(&save_path, test_support::valid_save_bytes(20, 30)).unwrap();

        // Poll (generous window) until BOTH effects appear — don't assert on
        // exact timing, just that the change is processed.
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut done = false;
        while Instant::now() < deadline {
            if status_path.exists() && count_savs(&snapshots_dir) >= 1 {
                done = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        assert!(
            done,
            "watcher did not produce status.json + a snapshot within 5s \
(status exists: {}, snapshots: {})",
            status_path.exists(),
            count_savs(&snapshots_dir)
        );

        // The resolved party proves the full pipeline (parse → status) ran.
        let text = std::fs::read_to_string(&status_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["party"][0]["species"], "MEWTWO");

        // Keep the TempDir alive until assertions finish.
        drop(dir);
    }

    fn count_savs(dir: &std::path::Path) -> usize {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return 0;
        };
        entries
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|x| x == "sav"))
            .count()
    }
}
