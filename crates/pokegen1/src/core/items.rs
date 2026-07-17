//! Parse Gen-1 item lists (bag + PC).
//!
//! Gen-1 stores an item list as a run of `(item_id, quantity)` byte pairs
//! terminated by a `0xFF` sentinel. [`read_item_list`] is the reusable walker;
//! [`bag_items`] and [`pc_items`] are the two named callers.
//!
//! Ported from `reference/read_save.py`'s `read_items()`. No item-id -> name
//! resolution happens here: `pokegen1::core` stays free of game-specific lookup
//! tables (the Yellow Legacy overlay owns that).

use serde::{Deserialize, Serialize};

use crate::core::sram::{self, SaveData};

/// Runaway guard: the maximum number of `(item, qty)` pairs [`read_item_list`]
/// will walk before giving up. Adapts `read_save.py`'s `limit=60`. The real
/// caps are smaller (Legacy bag holds 41, PC ~50), so 64 comfortably covers a
/// valid list while still bounding a corrupt/unterminated one.
const MAX_ITEMS: usize = 64;

/// One `(item_id, quantity)` entry in an item list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    pub item_id: u8,
    pub quantity: u8,
}

/// Walk a `0xFF`-terminated run of `(item_id, quantity)` byte pairs beginning at
/// `start`, returning one [`ItemStack`] per pair.
///
/// Ported from `read_save.py`'s `read_items()`: stops at the `0xFF` terminator,
/// or after `MAX_ITEMS` pairs (the runaway guard) if no terminator is found.
pub fn read_item_list(save: &SaveData, start: usize) -> Vec<ItemStack> {
    let mut out = Vec::new();
    for i in 0..MAX_ITEMS {
        let item_id = save.read_u8(start + i * 2);
        if item_id == 0xFF {
            break;
        }
        out.push(ItemStack {
            item_id,
            quantity: save.read_u8(start + i * 2 + 1),
        });
    }
    out
}

/// Parse the bag item list.
pub fn bag_items(save: &SaveData) -> Vec<ItemStack> {
    read_item_list(save, sram::BAG_ITEMS)
}

/// Parse the PC item list.
///
/// The byte at [`sram::PC_ITEMS`] is a leading COUNT byte; the `(item, qty)`
/// pairs start at [`sram::PC_ITEMS_DATA`] (one byte later), matching
/// `read_save.py`'s `read_items(d, O_PC + 1)`.
pub fn pc_items(save: &SaveData) -> Vec<ItemStack> {
    read_item_list(save, sram::PC_ITEMS_DATA)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_support::{blank_sram, seed};

    #[test]
    fn bag_items_reads_pairs_until_terminator() {
        let mut bytes = blank_sram();
        // Ultra Ball(2) x12, Rare Candy(40) x5, then 0xFF terminator.
        seed(&mut bytes, sram::BAG_ITEMS, &[2, 12, 40, 5, 0xFF]);
        let save = SaveData::new(bytes);
        assert_eq!(
            bag_items(&save),
            vec![
                ItemStack { item_id: 2, quantity: 12 },
                ItemStack { item_id: 40, quantity: 5 },
            ]
        );
    }

    #[test]
    fn pc_items_skips_leading_count_byte() {
        let mut bytes = blank_sram();
        // Leading COUNT byte (3), then id 20 x10, id 4 x99, then 0xFF.
        // If the count byte weren't skipped, the first "item" would be id 3.
        seed(&mut bytes, sram::PC_ITEMS, &[3, 20, 10, 4, 99, 0xFF]);
        let save = SaveData::new(bytes);
        assert_eq!(
            pc_items(&save),
            vec![
                ItemStack { item_id: 20, quantity: 10 },
                ItemStack { item_id: 4, quantity: 99 },
            ]
        );
    }

    #[test]
    fn empty_list_returns_empty_vec() {
        let mut bytes = blank_sram();
        // 0xFF immediately at the start of the bag list.
        seed(&mut bytes, sram::BAG_ITEMS, &[0xFF]);
        let save = SaveData::new(bytes);
        assert_eq!(bag_items(&save), vec![]);
    }

    #[test]
    fn quantity_of_0xff_is_not_a_terminator() {
        let mut bytes = blank_sram();
        // A legit stack with quantity 255, then the real 0xFF terminator on the
        // NEXT item-id byte. Guards the invariant that the sentinel is checked on
        // the item-id position, never on the quantity.
        seed(&mut bytes, sram::BAG_ITEMS, &[10, 0xFF, 0xFF]);
        let save = SaveData::new(bytes);
        assert_eq!(
            bag_items(&save),
            vec![ItemStack { item_id: 10, quantity: 255 }]
        );
    }

    #[test]
    fn unterminated_list_is_capped_by_runaway_guard() {
        let mut bytes = blank_sram();
        // Fill well past MAX_ITEMS pairs with non-0xFF bytes and no terminator.
        let filler = vec![1u8; MAX_ITEMS * 2 + 20];
        seed(&mut bytes, sram::BAG_ITEMS, &filler);
        let save = SaveData::new(bytes);
        assert_eq!(bag_items(&save).len(), MAX_ITEMS);
    }
}
