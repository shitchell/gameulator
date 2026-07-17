//! Top-level Gen-1 save model and the public [`parse_save`] entry point.
//!
//! [`parse_save`] is THE public Model entry point the app controller calls: it
//! gates on buffer length, wraps the bytes in a [`SaveData`], and assembles the
//! game-agnostic [`Save`] from the per-field parsers (header, party, items,
//! checksum).

use serde::{Deserialize, Serialize};

use crate::core::error::ParseError;
use crate::core::header::{self, Playtime};
use crate::core::items::{self, ItemStack};
use crate::core::party;
use crate::core::pokemon::Pokemon;
use crate::core::sram::{self, SaveData};
use crate::core::{checksum, header::trainer_name};

/// A parsed Gen-1 save: the game-agnostic Model.
///
/// Deliberately free of game-specific fields (e.g. Yellow-Legacy `wDifficulty`);
/// those belong to the game overlay, not this core Model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Save {
    pub trainer: String,
    pub playtime: Playtime,
    pub party: Vec<Pokemon>,
    pub bag: Vec<ItemStack>,
    pub pc: Vec<ItemStack>,
    /// Whether the stored main-data checksum verified. A `false` here is NOT a
    /// parse error — [`parse_save`] still returns `Ok`; the sync watcher decides
    /// what to do with an unverified save.
    pub checksum_ok: bool,
}

/// Parse a raw Gen-1 save buffer into a [`Save`].
///
/// # Length gate
/// Validates `bytes.len() >= sram::SAVE_LEN` FIRST — before constructing a
/// [`SaveData`] or touching any offset — because every accessor indexes the
/// buffer directly and would otherwise panic on a short/corrupt one. Uses `>=`
/// (not `==`): a save may carry a trailing RTC/footer and be longer.
///
/// # Errors
/// - [`ParseError::TruncatedSave`] if the buffer is shorter than a full SRAM dump.
/// - [`ParseError::InvalidPartyCount`] if the party-count byte is out of range.
///
/// A BAD checksum is deliberately NOT an error: it surfaces as
/// [`Save::checksum_ok`] `== false` on an otherwise-`Ok` result.
pub fn parse_save(bytes: Vec<u8>) -> Result<Save, ParseError> {
    if bytes.len() < sram::SAVE_LEN {
        return Err(ParseError::TruncatedSave {
            expected: sram::SAVE_LEN,
            got: bytes.len(),
        });
    }

    let save = SaveData::new(bytes);

    Ok(Save {
        trainer: trainer_name(&save),
        playtime: header::playtime(&save),
        party: party::parse_party(&save)?,
        bag: items::bag_items(&save),
        pc: items::pc_items(&save),
        checksum_ok: checksum::verify_checksum(&save),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::pokemon::MoveSlot;
    use crate::core::test_support::{blank_sram, seed};

    /// Recompute the main-data checksum the same way the checksum module/tests
    /// do (sum the covered range, ones-complement the low byte).
    fn expected_checksum(buf: &[u8]) -> u8 {
        let mut sum: u8 = 0;
        for &b in &buf[sram::MAIN_DATA_START..=sram::MAIN_DATA_END] {
            sum = sum.wrapping_add(b);
        }
        sum ^ 0xFF
    }

    /// Build a synthetic full save: trainer, playtime, one party mon with a
    /// nickname, bag + PC items. Does NOT write the checksum (callers decide).
    fn full_save_bytes() -> Vec<u8> {
        let mut buf = blank_sram();

        // Trainer name "RED".
        seed(&mut buf, sram::NAME, &[0x92, 0x87, 0x80, 0x94, 0x8D, 0x50]);

        // Playtime 24h 12m.
        seed(&mut buf, sram::PLAYTIME_HOURS, &[24]);
        seed(&mut buf, sram::PLAYTIME_MINUTES, &[12]);

        // One-mon party.
        seed(&mut buf, sram::PARTY_COUNT, &[1]);
        seed(&mut buf, sram::PARTY_SPECIES, &[131]); // MEWTWO
        let base = sram::PARTY_DATA;
        seed(&mut buf, base, &[131]); // species
        seed(&mut buf, base + 0x01, &[0x00, 0x14]); // cur HP = 20
        seed(&mut buf, base + 0x04, &[0x40]); // status: PARALYZE
        seed(&mut buf, base + 0x08, &[85, 0, 0, 0]); // one move id 85
        seed(&mut buf, base + 0x1D, &[0x0F, 0, 0, 0]); // pp 15
        seed(&mut buf, base + 0x21, &[100]); // level
        seed(&mut buf, base + 0x22, &[0x00, 0x28]); // max HP = 40
        seed(&mut buf, base + 0x24, &[0x00, 0xA0]); // atk 160
        seed(&mut buf, base + 0x26, &[0x00, 0x96]); // def 150
        seed(&mut buf, base + 0x28, &[0x00, 0x8C]); // spd 140
        seed(&mut buf, base + 0x2A, &[0x00, 0xC8]); // spc 200

        // Nickname "SPARKY".
        seed(
            &mut buf,
            sram::NICKNAMES,
            &[0x92, 0x8F, 0x80, 0x91, 0x8A, 0x98, 0x50],
        );

        // Bag: Ultra Ball(2) x12, Rare Candy(40) x5.
        seed(&mut buf, sram::BAG_ITEMS, &[2, 12, 40, 5, 0xFF]);
        // PC: leading count byte 2, then id 20 x10, id 4 x99.
        seed(&mut buf, sram::PC_ITEMS, &[2, 20, 10, 4, 99, 0xFF]);

        buf
    }

    #[test]
    fn parses_full_save_with_valid_checksum() {
        let mut buf = full_save_bytes();
        let sum = expected_checksum(&buf);
        seed(&mut buf, sram::MAIN_CHECKSUM, &[sum]);

        let save = parse_save(buf).expect("full save should parse");

        assert_eq!(save.trainer, "RED");
        assert_eq!(
            save.playtime,
            Playtime {
                hours: 24,
                minutes: 12
            }
        );

        assert_eq!(save.party.len(), 1);
        let mon = &save.party[0];
        assert_eq!(mon.species_id, 131);
        assert_eq!(mon.level, 100);
        assert_eq!(mon.hp, 20);
        assert_eq!(mon.max_hp, 40);
        assert_eq!(mon.atk, 160);
        assert_eq!(mon.def, 150);
        assert_eq!(mon.spd, 140);
        assert_eq!(mon.spc, 200);
        assert_eq!(mon.nickname, Some("SPARKY".to_string()));
        assert_eq!(
            mon.moves,
            vec![MoveSlot {
                move_id: 85,
                pp: 15,
                pp_ups: 0,
                slot: 0
            }]
        );
        assert!(mon.status.paralyze);

        assert_eq!(
            save.bag,
            vec![
                ItemStack {
                    item_id: 2,
                    quantity: 12
                },
                ItemStack {
                    item_id: 40,
                    quantity: 5
                },
            ]
        );
        assert_eq!(
            save.pc,
            vec![
                ItemStack {
                    item_id: 20,
                    quantity: 10
                },
                ItemStack {
                    item_id: 4,
                    quantity: 99
                },
            ]
        );

        assert!(save.checksum_ok);
    }

    #[test]
    fn bad_checksum_is_ok_with_flag_false() {
        let mut buf = full_save_bytes();
        let sum = expected_checksum(&buf);
        seed(&mut buf, sram::MAIN_CHECKSUM, &[sum]);
        // Corrupt one in-range byte AFTER writing the checksum.
        buf[sram::MAIN_DATA_START + 50] ^= 0xFF;

        let save = parse_save(buf).expect("bad checksum must not be a parse error");
        assert!(!save.checksum_ok);
        // The rest still parsed.
        assert_eq!(save.trainer, "RED");
        assert_eq!(save.party.len(), 1);
    }

    #[test]
    fn truncated_buffer_errors_without_panic() {
        let err = parse_save(vec![0u8; 100]).unwrap_err();
        assert_eq!(
            err,
            ParseError::TruncatedSave {
                expected: 0x8000,
                got: 100
            }
        );
    }

    #[test]
    fn invalid_party_count_errors() {
        let mut buf = blank_sram(); // party count byte defaults to 0
        seed(&mut buf, sram::PARTY_COUNT, &[0]);
        let err = parse_save(buf).unwrap_err();
        assert_eq!(err, ParseError::InvalidPartyCount(0));
    }
}
