//! The per-change pipeline: validate → snapshot → regression → status.
//!
//! `process_save` is the watcher's per-change action (Task 8's notify loop reads
//! the settled file and calls this, then LOGS the returned [`Outcome`]). It never
//! prints itself, so it stays fully testable: parse ONCE and branch, quarantine
//! with a SPECIFIC reason, and on the valid path ALWAYS write the snapshot (even
//! on a regression — keep-all, alarm-not-delete) before writing `status.json`.

use std::path::PathBuf;

use crate::{regression::RegressionCheck, snapshot, status, Config};
use pokegen1::core::checksum::compute_checksum;
use pokegen1::core::sram::{self, SaveData};
use pokegen1::{parse_save, GameData};

/// The result of processing one settled save-file change. Returned to the caller
/// (the watcher, Task 8) which LOGS based on it — `process_save` itself does not
/// print, keeping it fully testable.
#[derive(Debug)]
pub enum Outcome {
    /// The file did not pass validation and was skipped. `reason` is specific
    /// (a parse error, or a checksum mismatch with the computed vs stored bytes).
    Quarantined { reason: String },
    /// A valid save was snapshotted. `regression` is `Accept` normally, or
    /// `Regression{..}` if this save is behind the latest snapshot (a stale-device
    /// clobber — the caller alarms loudly; the snapshot is STILL written).
    Applied {
        snapshot: PathBuf,
        regression: RegressionCheck,
    },
}

/// Run one settled save-file change through the pipeline. Parses `bytes` ONCE and
/// branches: a parse error or a bad checksum yields [`Outcome::Quarantined`] with
/// a specific reason and NO side effects; a valid save is snapshotted (always,
/// even on a regression) and summarized to `status.json`, yielding
/// [`Outcome::Applied`] carrying the [`RegressionCheck`]. `stamp` is injected and
/// reused for both the snapshot filename and the status `last_change`.
///
/// The snapshot is written before `status.json`, so if the (atomic) status write
/// fails the durable save backup is already committed and `status.json` self-heals
/// on the next accepted save — callers should NOT add rollback logic.
pub fn process_save(
    cfg: &Config,
    game: &dyn GameData,
    bytes: &[u8],
    stamp: &str,
) -> anyhow::Result<Outcome> {
    let save = match parse_save(bytes.to_vec()) {
        Err(e) => {
            return Ok(Outcome::Quarantined {
                reason: format!("parse failed: {e}"),
            });
        }
        Ok(save) => save,
    };

    if !save.checksum_ok {
        let computed = compute_checksum(&SaveData::new(bytes.to_vec()));
        let stored = bytes[sram::MAIN_CHECKSUM];
        return Ok(Outcome::Quarantined {
            reason: format!("checksum mismatch: computed {computed:#04x}, stored {stored:#04x}"),
        });
    }

    let incoming = save.playtime.hours as u32 * 60 + save.playtime.minutes as u32;
    // Baseline is the HIGH-WATER mark (max playtime across ALL snapshots), not the
    // newest snapshot — so ANY save behind the furthest-progressed state alarms,
    // even after a stale device has already written a low-playtime snapshot.
    let baseline = snapshot::max_snapshot_playtime(&cfg.snapshots_dir)?;
    let regression = crate::regression::check(incoming, baseline);

    // Snapshot is ALWAYS written on the valid path — even on a Regression;
    // keep-all, alarm-not-delete. The caller decides how loudly to warn.
    let snap = snapshot::write_snapshot(&cfg.snapshots_dir, bytes, stamp)?;
    status::write_status(&cfg.status_path, &save, game, stamp, Some(&snap))?;

    Ok(Outcome::Applied {
        snapshot: snap,
        regression,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::valid_save_bytes;

    fn cfg(dir: &std::path::Path) -> Config {
        Config::for_game_dir(dir)
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

    #[test]
    fn corrupt_parse_error_is_quarantined_with_no_side_effects() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = cfg(dir.path());
        let game = app::game_data(app::GameId::YellowLegacy);

        let outcome =
            process_save(&cfg, game.as_ref(), &[0u8; 100], "2026-01-01T00-00-00.000Z").unwrap();

        match outcome {
            Outcome::Quarantined { reason } => {
                assert!(
                    reason.contains("parse failed"),
                    "reason should mention parse failure: {reason}"
                );
            }
            other => panic!("expected Quarantined, got {other:?}"),
        }

        assert_eq!(
            count_savs(&cfg.snapshots_dir),
            0,
            "no snapshot must be written"
        );
        assert!(!cfg.status_path.exists(), "status.json must not be written");
    }

    #[test]
    fn corrupt_checksum_is_quarantined_with_computed_and_stored() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = cfg(dir.path());
        let game = app::game_data(app::GameId::YellowLegacy);

        let mut bytes = valid_save_bytes(20, 0);
        // Flip an in-range byte WITHOUT recomputing the checksum → checksum breaks.
        bytes[sram::MAIN_DATA_START + 100] ^= 0xFF;

        let outcome =
            process_save(&cfg, game.as_ref(), &bytes, "2026-01-01T00-00-00.000Z").unwrap();

        match outcome {
            Outcome::Quarantined { reason } => {
                assert!(
                    reason.contains("checksum mismatch"),
                    "reason should mention checksum mismatch: {reason}"
                );
            }
            other => panic!("expected Quarantined, got {other:?}"),
        }

        assert_eq!(
            count_savs(&cfg.snapshots_dir),
            0,
            "no snapshot must be written"
        );
        assert!(!cfg.status_path.exists(), "status.json must not be written");
    }

    #[test]
    fn valid_save_is_applied_with_accept_and_side_effects() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = cfg(dir.path());
        let game = app::game_data(app::GameId::YellowLegacy);

        let outcome = process_save(
            &cfg,
            game.as_ref(),
            &valid_save_bytes(20, 0),
            "2026-01-01T00-00-00.000Z",
        )
        .unwrap();

        match outcome {
            Outcome::Applied {
                regression,
                snapshot,
            } => {
                assert_eq!(regression, RegressionCheck::Accept);
                assert!(snapshot.exists(), "snapshot file must exist");
            }
            other => panic!("expected Applied, got {other:?}"),
        }

        assert_eq!(
            count_savs(&cfg.snapshots_dir),
            1,
            "a snapshot must be written"
        );
        assert!(cfg.status_path.exists(), "status.json must be written");

        let text = std::fs::read_to_string(&cfg.status_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["party"][0]["species"], "MEWTWO");
    }

    #[test]
    fn newer_then_older_is_applied_regression_but_snapshot_still_kept() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = cfg(dir.path());
        let game = app::game_data(app::GameId::YellowLegacy);

        // First: 20h (1200 min).
        process_save(
            &cfg,
            game.as_ref(),
            &valid_save_bytes(20, 0),
            "2026-01-01T00-00-00.000Z",
        )
        .unwrap();

        // Then: 10h (600 min) — behind the latest snapshot → a regression.
        let outcome = process_save(
            &cfg,
            game.as_ref(),
            &valid_save_bytes(10, 0),
            "2026-02-01T00-00-00.000Z",
        )
        .unwrap();

        match outcome {
            Outcome::Applied {
                regression,
                snapshot,
            } => {
                assert_eq!(
                    regression,
                    RegressionCheck::Regression {
                        incoming: 600,
                        latest: 1200,
                    }
                );
                assert!(
                    snapshot.exists(),
                    "the stale snapshot must still be written"
                );
            }
            other => panic!("expected Applied, got {other:?}"),
        }

        // Both snapshots kept — keep-all, alarm-not-delete.
        assert_eq!(
            count_savs(&cfg.snapshots_dir),
            2,
            "both snapshots must be kept (stale one is NOT deleted)"
        );
    }

    #[test]
    fn baseline_is_high_water_mark_so_second_stale_save_still_alarms() {
        // The I1 regression scenario: after a stale device writes a low-playtime
        // snapshot, a further save STILL behind the furthest-progressed state must
        // ALSO alarm. The baseline is the high-water mark (1200), not the newest
        // snapshot — so the guard does NOT go silent once the baseline regresses.
        let dir = tempfile::tempdir().unwrap();
        let cfg = cfg(dir.path());
        let game = app::game_data(app::GameId::YellowLegacy);

        // 1. 20h (1200 min) — the furthest-progressed state. Accepted.
        let outcome = process_save(
            &cfg,
            game.as_ref(),
            &valid_save_bytes(20, 0),
            "2026-01-01T00-00-00.000Z",
        )
        .unwrap();
        match outcome {
            Outcome::Applied { regression, .. } => {
                assert_eq!(regression, RegressionCheck::Accept);
            }
            other => panic!("expected Applied/Accept, got {other:?}"),
        }

        // 2. 11h (660 min) — a stale device, behind 1200 → regression vs 1200.
        let outcome = process_save(
            &cfg,
            game.as_ref(),
            &valid_save_bytes(11, 0),
            "2026-02-01T00-00-00.000Z",
        )
        .unwrap();
        match outcome {
            Outcome::Applied { regression, .. } => {
                assert_eq!(
                    regression,
                    RegressionCheck::Regression {
                        incoming: 660,
                        latest: 1200,
                    }
                );
            }
            other => panic!("expected Applied/Regression, got {other:?}"),
        }

        // 3. 12h (720 min) — STILL behind the 1200 high-water mark. This MUST
        //    also alarm with latest == 1200. Before the fix, the baseline had
        //    regressed to 660 (the newest snapshot), so 720 would wrongly Accept.
        let outcome = process_save(
            &cfg,
            game.as_ref(),
            &valid_save_bytes(12, 0),
            "2026-03-01T00-00-00.000Z",
        )
        .unwrap();
        match outcome {
            Outcome::Applied { regression, .. } => {
                assert_eq!(
                    regression,
                    RegressionCheck::Regression {
                        incoming: 720,
                        latest: 1200,
                    },
                    "720 is still behind the 1200 high-water mark — the baseline \
                     must NOT have regressed to 660"
                );
            }
            other => panic!("expected Applied/Regression, got {other:?}"),
        }

        // All three snapshots kept — keep-all.
        assert_eq!(
            count_savs(&cfg.snapshots_dir),
            3,
            "all three snapshots must be kept (keep-all)"
        );
    }
}
