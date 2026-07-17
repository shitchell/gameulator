//! Composable dashboard components. Each renders via CSS classes only — no
//! inline colors/sizes (the token CSS is Task 8). The one exception is the HP
//! bar's `width: N%` inline style, which is data (the fill ratio), not theming.

mod hp_bar;
mod info_header;
mod item_list;
mod move_list;
mod party_card;
mod status_badges;

pub use hp_bar::HpBar;
pub use info_header::InfoHeader;
pub use item_list::ItemList;
pub use move_list::MoveList;
pub use party_card::PartyCard;
pub use status_badges::StatusBadges;
