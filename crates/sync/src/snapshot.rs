//! Timestamped snapshot writer. The pure `write_snapshot` takes an injected
//! stamp so pipeline tests stay deterministic; `stamp_now` (real clock) is only
//! called by the watcher.

use anyhow::Context;
use std::path::{Path, PathBuf};

/// Write `bytes` as a snapshot named `{stamp}.sav` in `dir` (creating `dir` if
/// missing). Returns the written path. `stamp` is injected (not read from the
/// clock) so tests are deterministic; the watcher passes `stamp_now()`.
pub fn write_snapshot(dir: &Path, bytes: &[u8], stamp: &str) -> anyhow::Result<PathBuf> {
    std::fs::create_dir_all(dir)
        .with_context(|| format!("creating snapshots dir {}", dir.display()))?;
    let path = dir.join(format!("{stamp}.sav"));
    std::fs::write(&path, bytes).with_context(|| format!("writing snapshot {}", path.display()))?;
    Ok(path)
}

/// A filesystem-safe, chronologically-sortable UTC timestamp for snapshot names,
/// e.g. `2026-07-17T14-30-00.123Z`. NO colons (not path-safe on all filesystems);
/// millisecond precision makes a same-name collision effectively impossible at
/// save cadence (uniqueness is not *enforced* — two writes in the same ms would
/// overwrite). Only the watcher calls this — the pure `write_snapshot` takes the
/// stamp as a param.
pub fn stamp_now() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H-%M-%S%.3fZ")
        .to_string()
}

/// The HIGH-WATER-MARK playtime (minutes) across ALL snapshots in `dir`, or
/// `None` if there are none. This is the regression baseline: an incoming save
/// behind the furthest-progressed snapshot is a stale-device clobber, so we
/// compare against the MAX playtime ever snapshotted — NOT merely the newest by
/// timestamp (a stale save writes a low-playtime *newest* snapshot; comparing to
/// the newest would let the baseline regress and silence the alarm).
///
/// Cost is O(N-snapshots) parses per incoming save — acceptable at save cadence
/// with keep-all for now; a cached high-water file or snapshot pruning (deferred)
/// is the future optimization if N grows large.
pub fn max_snapshot_playtime(dir: &Path) -> anyhow::Result<Option<u32>> {
    // Missing dir (or otherwise unreadable) → no snapshots yet, which is normal.
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(None);
    };

    // Parse every *.sav and take the MAX playtime. `filter_map` skips a single
    // unreadable/unparseable snapshot rather than aborting the whole max — these
    // are files we wrote, so they should parse, but one bad file must not silence
    // the guard. Returns `None` only when there are no parseable snapshots.
    let max = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "sav"))
        .filter_map(|path| {
            let bytes = std::fs::read(&path).ok()?;
            let save = pokegen1::parse_save(bytes).ok()?;
            Some(save.playtime.hours as u32 * 60 + save.playtime.minutes as u32)
        })
        .max();

    Ok(max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_named_snapshot_with_exact_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_snapshot(dir.path(), b"hello", "2026-07-17T14-30-00.000Z").unwrap();

        assert_eq!(path, dir.path().join("2026-07-17T14-30-00.000Z.sav"));
        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
    }

    #[test]
    fn creates_missing_dir() {
        let dir = tempfile::tempdir().unwrap();
        let snapshots = dir.path().join("snapshots");
        assert!(!snapshots.exists());

        let path = write_snapshot(&snapshots, b"data", "2026-07-17T14-30-00.000Z").unwrap();

        assert!(path.exists());
    }

    #[test]
    fn max_snapshot_playtime_picks_max_not_newest_by_name() {
        let dir = tempfile::tempdir().unwrap();
        // The older-by-NAME snapshot has the LARGER playtime (1200 min); the
        // newer-by-NAME has the SMALLER playtime (600 min). The high-water mark
        // must be 1200 — NOT 600 (the newest by name). This is the crux of the
        // I1 fix: a stale device writing a low-playtime *newest* snapshot must
        // NOT drop the regression baseline.
        write_snapshot(
            dir.path(),
            &crate::test_support::valid_save_bytes(20, 0),
            "2026-01-01T00-00-00.000Z",
        )
        .unwrap();
        write_snapshot(
            dir.path(),
            &crate::test_support::valid_save_bytes(10, 0),
            "2026-06-01T00-00-00.000Z",
        )
        .unwrap();

        assert_eq!(
            max_snapshot_playtime(dir.path()).unwrap(),
            Some(20 * 60),
            "must return the MAX playtime (1200), not the newest-by-name (600)"
        );
    }

    #[test]
    fn max_snapshot_playtime_empty_dir_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(max_snapshot_playtime(dir.path()).unwrap(), None);
    }

    #[test]
    fn max_snapshot_playtime_missing_dir_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        assert!(!missing.exists());
        assert_eq!(max_snapshot_playtime(&missing).unwrap(), None);
    }

    #[test]
    fn max_snapshot_playtime_ignores_non_sav_files() {
        let dir = tempfile::tempdir().unwrap();
        write_snapshot(
            dir.path(),
            &crate::test_support::valid_save_bytes(5, 15),
            "2026-03-01T00-00-00.000Z",
        )
        .unwrap();
        // A non-.sav file with a lexicographically-larger name must be ignored.
        std::fs::write(dir.path().join("zzz-notes.txt"), b"not a save").unwrap();

        assert_eq!(
            max_snapshot_playtime(dir.path()).unwrap(),
            Some(5 * 60 + 15)
        );
    }

    #[test]
    fn stamp_now_is_path_safe_and_sortable() {
        let stamp = stamp_now();

        assert!(
            !stamp.contains(':'),
            "stamp must not contain colons: {stamp}"
        );
        assert!(stamp.ends_with('Z'), "stamp must end with Z: {stamp}");
        // Shape: `YYYY-...` — starts with 4 digits then a dash.
        let bytes = stamp.as_bytes();
        assert!(
            bytes.len() >= 5 && bytes[..4].iter().all(u8::is_ascii_digit) && bytes[4] == b'-',
            "stamp must start with 4 digits + dash: {stamp}"
        );
    }
}
