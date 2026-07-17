//! Save-sync watcher: validates, snapshots, and regression-guards the synced
//! .sav; writes status.json. Logic in testable functions; the notify loop is
//! thin.

use std::path::{Path, PathBuf};
use std::time::Duration;

pub mod process;
pub mod regression;
pub mod snapshot;
pub mod status;
pub mod watch;

#[cfg(test)]
pub(crate) mod test_support;

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

impl Config {
    /// Standard layout derived from a saves directory:
    /// `save_path` = the first `*.sav` in `saves_dir` (or `saves_dir/save.sav` as a
    /// documented default if none exists yet), `snapshots_dir = saves_dir/snapshots`,
    /// `status_path = saves_dir/status.json`, `debounce = 2s`.
    pub fn for_game_dir(saves_dir: &Path) -> Self {
        let save_path = first_sav(saves_dir).unwrap_or_else(|| saves_dir.join("save.sav"));
        Self {
            save_path,
            snapshots_dir: saves_dir.join("snapshots"),
            status_path: saves_dir.join("status.json"),
            debounce: Duration::from_secs(2),
        }
    }
}

/// The first `*.sav` file in `dir` (sorted for determinism), if any.
fn first_sav(dir: &Path) -> Option<PathBuf> {
    let mut savs: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "sav").unwrap_or(false))
        .collect();
    savs.sort();
    savs.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn for_game_dir_picks_seeded_sav_and_derives_subpaths() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path();
        let seeded = saves_dir.join("foo.sav");
        std::fs::write(&seeded, b"").unwrap();

        let cfg = Config::for_game_dir(saves_dir);

        assert_eq!(cfg.save_path, seeded);
        assert_eq!(cfg.snapshots_dir, saves_dir.join("snapshots"));
        assert_eq!(cfg.status_path, saves_dir.join("status.json"));
        assert_eq!(cfg.debounce, Duration::from_secs(2));
    }

    #[test]
    fn for_game_dir_empty_dir_falls_back_to_save_sav() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config::for_game_dir(dir.path());
        assert_eq!(cfg.save_path, dir.path().join("save.sav"));
    }

    #[test]
    fn for_game_dir_picks_lexicographically_first_sav() {
        // Determinism guard: read_dir order is FS-dependent, so first_sav sorts.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("b.sav"), b"").unwrap();
        std::fs::write(dir.path().join("a.sav"), b"").unwrap();
        let cfg = Config::for_game_dir(dir.path());
        assert_eq!(cfg.save_path, dir.path().join("a.sav"));
    }
}
