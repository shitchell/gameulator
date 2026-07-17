//! `pokegen1` — the Generation 1 Pokémon save/ROM **Model**.
//!
//! Parses a raw Gen-1 SRAM save (`.sav`/`.srm`) into a game-agnostic [`Save`]
//! (trainer, playtime, party, bag/PC items, checksum status). [`parse_save`] is
//! the single public entry point the app controller calls; the length gate it
//! runs is the precondition every direct-indexing accessor relies on.
//!
//! Game-specific concerns (species/move/item name tables, Yellow-Legacy overlay
//! fields) live OUTSIDE this crate — `core` stays free of lookup tables.
//!
//! The lower-level [`core`] modules remain public so downstream crates (e.g. the
//! sync watcher) can reach fns like [`core::checksum::verify_checksum`].

pub mod core;

pub use core::error::ParseError;
pub use core::header::Playtime;
pub use core::items::ItemStack;
pub use core::pokemon::{MoveSlot, Pokemon, Status};
pub use core::save::{parse_save, Save};
