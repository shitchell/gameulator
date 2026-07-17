//! `gameulator-sync` entrypoint. Real watcher wiring lands in a later task; for
//! now this builds a [`Config`] for the standard saves layout and prints it.

use std::path::Path;

use sync::Config;

fn main() -> anyhow::Result<()> {
    let cfg = Config::for_game_dir(Path::new("games/Pokemon/Yellow Legacy/saves"));
    println!("{cfg:?}");
    Ok(())
}
