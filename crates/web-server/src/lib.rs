//! axum backend for the Gameulator web dashboard: serves the built WASM
//! frontend + a small JSON API (/api/status, /api/config) over the sync
//! watcher's status.json.

use std::path::PathBuf;

use clap::Parser;

/// Runtime config for the web dashboard server.
#[derive(Debug, Clone, Parser)]
#[command(name = "gameulator-web")]
pub struct WebConfig {
    /// Path to the sync watcher's status.json.
    #[arg(long, default_value = "games/Pokemon/Yellow Legacy/saves/status.json")]
    pub status_path: PathBuf,
    /// Directory of the built frontend assets (trunk dist).
    #[arg(long, default_value = "crates/web/dist")]
    pub dist_dir: PathBuf,
    /// Port to bind.
    #[arg(long, default_value_t = 8770)]
    pub port: u16,
    /// Poll interval (ms) the frontend uses to re-fetch /api/status.
    #[arg(long, default_value_t = 2000)]
    pub poll_ms: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_config_defaults() {
        let cfg = WebConfig::parse_from(["gameulator-web"]);
        assert_eq!(
            cfg.status_path,
            PathBuf::from("games/Pokemon/Yellow Legacy/saves/status.json")
        );
        assert_eq!(cfg.dist_dir, PathBuf::from("crates/web/dist"));
        assert_eq!(cfg.port, 8770);
        assert_eq!(cfg.poll_ms, 2000);
    }
}
