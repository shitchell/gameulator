//! `status.json` writer — the Milestone-3 web seam.
//!
//! After each accepted save the pipeline (Task 7) writes a `status.json`
//! summarizing the parsed save plus change metadata. This is the file the
//! Milestone-3 web dashboard polls (it re-fetches this file every ~2s). The
//! writer reuses the `app` controller (`save_info` + `party_summary`) so the
//! trainer/party/playtime rendering lives in ONE place — no logic duplicated
//! here.

use std::path::Path;

use anyhow::Context;

// All types come from `app` (which re-exports the pokegen1 ones its API needs),
// so this seam depends only on the controller boundary. `StatusView` is the
// shared web↔sync contract, owned by `app`: sync writes it, the browser reads it.
use app::{GameData, Save, StatusView};

/// Build + write `status.json` from a parsed save. Reuses the `app` controller
/// (save_info + party_summary) so the JSON contract is exactly the app DTOs +
/// change metadata — no logic duplicated here.
pub fn write_status(
    path: &Path,
    save: &Save,
    game: &dyn GameData,
    stamp: &str,
    snapshot: Option<&Path>,
) -> anyhow::Result<()> {
    let info = app::save_info(save);
    let status = StatusView {
        trainer: info.trainer,
        playtime: info.playtime,
        checksum_ok: info.checksum_ok,
        party: app::party_summary(save, game),
        last_change: stamp.to_string(),
        snapshot: snapshot.map(|p| p.display().to_string()),
    };
    let json = serde_json::to_string_pretty(&status)?;
    // Write atomically (temp + rename) so the M3 web view, which polls this file,
    // never sees a torn/partial read mid-write. rename is atomic on the same fs.
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp);
    std::fs::write(&tmp, json)
        .with_context(|| format!("writing status.json temp {}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("renaming status.json into place at {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support;

    #[test]
    fn writes_status_with_metadata_and_resolved_party() {
        let dir = tempfile::tempdir().unwrap();
        let status_path = dir.path().join("status.json");
        let snap_path = dir.path().join("snapshots/2026-07-17T14-30-00.000Z.sav");

        let save = pokegen1::parse_save(test_support::valid_save_bytes(20, 30)).unwrap();
        let game = app::game_data(app::GameId::YellowLegacy);

        write_status(
            &status_path,
            &save,
            game.as_ref(),
            "2026-07-17T14-30-00.000Z",
            Some(&snap_path),
        )
        .unwrap();

        let text = std::fs::read_to_string(&status_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();

        assert_eq!(v["playtime"]["hours"], 20);
        assert_eq!(v["playtime"]["minutes"], 30);
        assert_eq!(v["checksum_ok"], true);

        let party = v["party"].as_array().expect("party is an array");
        assert!(!party.is_empty(), "party should be non-empty");
        assert_eq!(party[0]["species"], "MEWTWO");

        assert_eq!(v["last_change"], "2026-07-17T14-30-00.000Z");
        assert_eq!(v["snapshot"], snap_path.display().to_string());
    }

    #[test]
    fn snapshot_none_serializes_as_null() {
        let dir = tempfile::tempdir().unwrap();
        let status_path = dir.path().join("status.json");

        let save = pokegen1::parse_save(test_support::valid_save_bytes(1, 2)).unwrap();
        let game = app::game_data(app::GameId::YellowLegacy);

        write_status(
            &status_path,
            &save,
            game.as_ref(),
            "2026-07-17T00-00-00.000Z",
            None,
        )
        .unwrap();

        let text = std::fs::read_to_string(&status_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(v["snapshot"].is_null());
    }
}
