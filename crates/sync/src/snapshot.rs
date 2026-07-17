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
    std::fs::write(&path, bytes)
        .with_context(|| format!("writing snapshot {}", path.display()))?;
    Ok(path)
}

/// A filesystem-safe, chronologically-sortable UTC timestamp for snapshot names,
/// e.g. `2026-07-17T14-30-00.123Z`. NO colons (not path-safe on all filesystems);
/// millisecond precision so two saves in the same second don't collide. Only the
/// watcher calls this — the pure `write_snapshot` takes the stamp as a param.
pub fn stamp_now() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S%.3fZ").to_string()
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
    fn stamp_now_is_path_safe_and_sortable() {
        let stamp = stamp_now();

        assert!(!stamp.contains(':'), "stamp must not contain colons: {stamp}");
        assert!(stamp.ends_with('Z'), "stamp must end with Z: {stamp}");
        // Shape: `YYYY-...` — starts with 4 digits then a dash.
        let bytes = stamp.as_bytes();
        assert!(
            bytes.len() >= 5 && bytes[..4].iter().all(u8::is_ascii_digit) && bytes[4] == b'-',
            "stamp must start with 4 digits + dash: {stamp}"
        );
    }
}
