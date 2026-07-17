//! Integration test against a REAL synced Gen-1 save, if one is present.
//!
//! Scans `games/Pokemon/Yellow Legacy/saves/` (relative to the workspace root)
//! for a `*.sav`. If none exists yet — the normal state until a save is synced —
//! it prints a note and returns (skips, does NOT fail). Once a real save lands
//! this auto-activates and exercises `parse_save` end-to-end.

use std::fs;
use std::path::PathBuf;

/// Locate the workspace-root saves directory. `CARGO_MANIFEST_DIR` points at the
/// `pokegen1` crate; the saves live two levels up (workspace root).
fn saves_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .join("..")
        .join("..")
        .join("games")
        .join("Pokemon")
        .join("Yellow Legacy")
        .join("saves")
}

#[test]
fn parses_real_save_if_present() {
    let dir = saves_dir();

    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => {
            eprintln!(
                "SKIP: no saves directory at {} yet — real-save test skipped.",
                dir.display()
            );
            return;
        }
    };

    let sav = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.extension().map(|x| x == "sav").unwrap_or(false));

    let Some(path) = sav else {
        eprintln!(
            "SKIP: no *.sav found in {} — real-save test skipped.",
            dir.display()
        );
        return;
    };

    let bytes = fs::read(&path).expect("read the .sav bytes");
    let save = pokegen1::parse_save(bytes)
        .unwrap_or_else(|e| panic!("parse_save failed on {}: {e}", path.display()));

    eprintln!(
        "parsed {}: trainer={:?} party={} checksum_ok={}",
        path.display(),
        save.trainer,
        save.party.len(),
        save.checksum_ok
    );
}
