//! `gameulator` — a thin CLI view over the `app` controller.
//!
//! Each subcommand asks `app` for a game resolver via `app::game_data`, loads
//! and parses the save via `app::load_save`, calls the matching controller op,
//! and renders the resulting DTOs. Game selection is a controller concern, so
//! this view names only an `app::GameId`, never a concrete overlay crate. All
//! logic (name resolution, nickname suppression, fainted and status derivation)
//! lives in `app`; this binary only formats.

mod render;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gameulator", about = "Inspect a Gen-1 Pokémon save")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show the party.
    Party {
        /// Path to the `.sav` file.
        save: PathBuf,
        /// Emit pretty JSON instead of text.
        #[arg(long)]
        json: bool,
        /// One diff-friendly line per Pokémon.
        #[arg(long)]
        compact: bool,
    },
    /// Show the bag items.
    Bag {
        /// Path to the `.sav` file.
        save: PathBuf,
        /// Emit pretty JSON instead of text.
        #[arg(long)]
        json: bool,
    },
    /// Show the PC items.
    Pc {
        /// Path to the `.sav` file.
        save: PathBuf,
        /// Emit pretty JSON instead of text.
        #[arg(long)]
        json: bool,
    },
    /// Show save-level info (trainer, playtime, checksum).
    Info {
        /// Path to the `.sav` file.
        save: PathBuf,
        /// Emit pretty JSON instead of text.
        #[arg(long)]
        json: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let game = app::game_data(app::GameId::YellowLegacy);

    match cli.command {
        Command::Party {
            save,
            json,
            compact,
        } => {
            let parsed = app::load_save(&save)?;
            let view = app::party_summary(&parsed, game.as_ref());
            if json {
                println!("{}", serde_json::to_string_pretty(&view)?);
            } else if compact {
                println!("{}", render::party_compact(&view));
            } else {
                println!("{}", render::party(&view));
            }
        }
        Command::Bag { save, json } => {
            let parsed = app::load_save(&save)?;
            let view = app::items_view(&parsed.bag, game.as_ref());
            if json {
                println!("{}", serde_json::to_string_pretty(&view)?);
            } else {
                println!("{}", render::items("BAG", &view));
            }
        }
        Command::Pc { save, json } => {
            let parsed = app::load_save(&save)?;
            let view = app::items_view(&parsed.pc, game.as_ref());
            if json {
                println!("{}", serde_json::to_string_pretty(&view)?);
            } else {
                println!("{}", render::items("PC", &view));
            }
        }
        Command::Info { save, json } => {
            let parsed = app::load_save(&save)?;
            let view = app::save_info(&parsed);
            if json {
                println!("{}", serde_json::to_string_pretty(&view)?);
            } else {
                println!("{}", render::info(&view));
            }
        }
    }

    Ok(())
}
