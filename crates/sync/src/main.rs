//! `gameulator-sync` entrypoint: watch a synced Gen-1 `.sav`, validate it,
//! snapshot it, guard against stale-device clobbers, and write `status.json`.
//!
//! Builds a [`sync::Config`] for the standard saves layout and hands it to
//! [`sync::watch::run`], which runs the startup pass then blocks forever
//! (Ctrl-C to stop).

use std::path::{Path, PathBuf};

use clap::Parser;

/// Watch a synced Gen-1 .sav: validate, snapshot, and guard against stale-device
/// clobbers, writing status.json for the (future) web view.
#[derive(Parser)]
#[command(name = "gameulator-sync")]
struct Cli {
    /// Directory holding the synced .sav (the Syncthing-shared saves folder).
    #[arg(long, default_value = "games/Pokemon/Yellow Legacy/saves")]
    saves_dir: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let cfg = sync::Config::for_game_dir(&cli.saves_dir);

    // Task-1 review carry-forward: log which .sav was selected, and warn if the
    // dir has more than one (for_game_dir silently picks the lexicographically-first).
    warn_if_multiple_savs(&cli.saves_dir, &cfg.save_path);

    eprintln!("[sync] watching:   {}", cfg.save_path.display());
    eprintln!("[sync] snapshots:  {}", cfg.snapshots_dir.display());
    eprintln!("[sync] status.json: {}", cfg.status_path.display());

    let game = app::game_data(app::GameId::YellowLegacy);
    sync::watch::run(cfg, game) // blocks forever (Ctrl-C to stop)
}

/// Warn if `saves_dir` holds more than one `*.sav` — `for_game_dir` silently
/// picks the lexicographically-first, so make the ambiguity visible. Says nothing
/// when there are 0 or 1 (the normal "watching:" line already covers those).
fn warn_if_multiple_savs(saves_dir: &Path, chosen: &Path) {
    let count = std::fs::read_dir(saves_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "sav"))
        .count();
    if count > 1 {
        eprintln!(
            "[sync] ⚠ {} .sav files in {} — watching only {}",
            count,
            saves_dir.display(),
            chosen.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use assert_cmd::Command;
    use predicates::str::contains;

    #[test]
    fn help_lists_saves_dir_flag_and_description() {
        Command::cargo_bin("gameulator-sync")
            .unwrap()
            .arg("--help")
            .assert()
            .success()
            .stdout(contains("--saves-dir"))
            .stdout(contains(
                "Watch a synced Gen-1 .sav: validate, snapshot, and guard",
            ));
    }
}
