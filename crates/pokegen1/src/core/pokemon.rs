//! Parse a single 44-byte party Pokémon struct.
//!
//! Ported EXACTLY from `reference/read_save.py`'s `extract()` — the authoritative
//! parse. Offsets below are RELATIVE to the struct base
//! `= sram::PARTY_DATA + slot * sram::PARTY_STRUCT_LEN` (slot 0-based).
//!
//! No species/move id -> name resolution happens here: `pokegen1::core` stays
//! free of game-specific lookup tables (a later variant-table task owns those).

use serde::{Deserialize, Serialize};

use crate::core::sram::{self, SaveData};

// ---- Party-struct field offsets (relative to the struct base) ----
// One conceptual change (a moved field) = one contiguous edit block.
const F_SPECIES: usize = 0x00;
const F_CUR_HP: usize = 0x01; // big-endian u16
const F_STATUS: usize = 0x04;
const F_MOVES: usize = 0x08; // 4 x u8 move ids at 0x08..=0x0B
const F_PP: usize = 0x1D; // 4 x u8 PP bytes at 0x1D..=0x20
// ⚠️ PARTY LEVEL LIVES AT 0x21, *NOT* 0x03.
// 0x03 is the BOXED-format level; read_party.py used 0x03 and is WRONG for the
// party format. read_save.py's `LEVEL_OFF = 0x21` is authoritative here. Reading
// 0x03 gives a plausible-but-wrong level and is the nastiest gotcha in this task.
const F_LEVEL: usize = 0x21;
const F_MAX_HP: usize = 0x22; // big-endian u16
const F_ATK: usize = 0x24; // big-endian u16
const F_DEF: usize = 0x26; // big-endian u16
const F_SPD: usize = 0x28; // big-endian u16
const F_SPC: usize = 0x2A; // big-endian u16

// ---- Status bitfield masks (byte at F_STATUS) ----
const STATUS_SLEEP_MASK: u8 = 0x07; // low 3 bits = sleep turn counter (0 = awake)
const STATUS_POISON: u8 = 0x08;
const STATUS_BURN: u8 = 0x10;
const STATUS_FREEZE: u8 = 0x20;
const STATUS_PARALYZE: u8 = 0x40;

// ---- PP byte layout ----
const PP_MASK: u8 = 0x3F; // low 6 bits = current PP
const PP_UPS_SHIFT: u8 = 6; // high 2 bits = PP Ups applied

/// Non-volatile status conditions decoded from the status byte.
///
/// Fainted is intentionally NOT here — it is derived from `hp == 0` (matching
/// read_save's "override conds with FAINTED when cur==0") and lives on
/// [`Pokemon`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    /// Sleep turn counter (0 = not asleep).
    pub sleep_turns: u8,
    pub poison: bool,
    pub burn: bool,
    pub freeze: bool,
    pub paralyze: bool,
}

impl Status {
    /// Whether ANY non-volatile status condition is present.
    ///
    /// A single home for the "has any status" predicate used by app/cli.
    pub fn any(&self) -> bool {
        self.sleep_turns > 0 || self.poison || self.burn || self.freeze || self.paralyze
    }
}

/// One occupied move slot (empty slots — move id 0 — are skipped from the
/// `Vec`, but the physical slot index is preserved in [`MoveSlot::slot`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveSlot {
    pub move_id: u8,
    pub pp: u8,
    pub pp_ups: u8,
    /// Physical move slot index (0–3) this move occupied in the struct.
    /// Preserved even though empty (id 0) slots are skipped from the `Vec`,
    /// so move-reorder / PP-per-slot logic can address the original slot.
    pub slot: u8,
}

/// A single party Pokémon, parsed from its 44-byte struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pokemon {
    pub species_id: u8,
    pub level: u8,
    pub hp: u16,
    pub max_hp: u16,
    pub atk: u16,
    pub def: u16,
    pub spd: u16,
    pub spc: u16,
    /// 0..=4 move slots; empty (id 0) slots are skipped, preserving order.
    pub moves: Vec<MoveSlot>,
    pub status: Status,
    /// In-game nickname, decoded from the party nickname region (which lives
    /// OUTSIDE the 44-byte struct). `parse_pokemon` always sets this to
    /// `None`; [`crate::core::party::parse_party`] populates it. `None` means
    /// the nickname region was empty.
    pub nickname: Option<String>,
}

impl Pokemon {
    /// Whether this Pokémon has fainted.
    ///
    /// Derived from `hp == 0` (matching read_save's "override conds with
    /// FAINTED when cur==0"). Kept as a method rather than a stored field
    /// because it is denormalized from `hp`; the app-layer view materializes
    /// it for JSON later.
    pub fn fainted(&self) -> bool {
        self.hp == 0
    }
}

/// Parse the party Pokémon in `slot` (0-based).
///
/// Precondition: `slot` refers to an occupied party slot within a valid save
/// buffer (the caller gates on `party_count`). Offsets are ported verbatim from
/// `read_save.py`'s `extract()`.
///
/// Deliberately `pub(crate)`: it is only ever called by
/// [`crate::core::party::parse_party`], which layers on the nickname (stored
/// outside the 44-byte struct). Calling it directly would yield a nickname-less
/// mon — a footgun — so it is not part of the public API.
pub(crate) fn parse_pokemon(save: &SaveData, slot: usize) -> Pokemon {
    let base = sram::PARTY_DATA + slot * sram::PARTY_STRUCT_LEN;

    let hp = save.read_u16_be(base + F_CUR_HP);

    let st = save.read_u8(base + F_STATUS);
    let status = Status {
        sleep_turns: st & STATUS_SLEEP_MASK,
        poison: st & STATUS_POISON != 0,
        burn: st & STATUS_BURN != 0,
        freeze: st & STATUS_FREEZE != 0,
        paralyze: st & STATUS_PARALYZE != 0,
    };

    // Four move ids at F_MOVES, four matching PP bytes at F_PP. A move id of 0
    // marks an empty slot and is skipped (read_save: `if not m: continue`).
    let mut moves = Vec::new();
    for j in 0..4 {
        let move_id = save.read_u8(base + F_MOVES + j);
        if move_id == 0 {
            continue;
        }
        let pp_byte = save.read_u8(base + F_PP + j);
        moves.push(MoveSlot {
            move_id,
            pp: pp_byte & PP_MASK,
            pp_ups: (pp_byte >> PP_UPS_SHIFT) & 0x3,
            slot: j as u8,
        });
    }

    Pokemon {
        species_id: save.read_u8(base + F_SPECIES),
        level: save.read_u8(base + F_LEVEL),
        hp,
        max_hp: save.read_u16_be(base + F_MAX_HP),
        atk: save.read_u16_be(base + F_ATK),
        def: save.read_u16_be(base + F_DEF),
        spd: save.read_u16_be(base + F_SPD),
        spc: save.read_u16_be(base + F_SPC),
        moves,
        status,
        // Nicknames live outside the 44-byte struct, so the struct parser
        // cannot know them; `parse_party` fills this in.
        nickname: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_support::{blank_sram, seed};

    /// Build a save with one fully-populated party struct at slot 0.
    ///
    /// Deliberately writes a DIFFERENT, wrong value at +0x03 (the boxed-format
    /// level offset) to prove `parse_pokemon` reads the party level from +0x21.
    fn save_with_mon() -> SaveData {
        let mut bytes = blank_sram();
        let base = sram::PARTY_DATA;

        seed(&mut bytes, base, &[131]); // species = MEWTWO (+0x00)
        seed(&mut bytes, base + 0x01, &[0x00, 0xFF]); // current HP = 255 (BE)
        seed(&mut bytes, base + 0x03, &[77]); // DECOY boxed-level: must NOT be read
        seed(&mut bytes, base + 0x04, &[0x40]); // status: PARALYZE only
        // moves: slot 0 id 85, slot 1 id 0 (empty -> skipped), slots 2/3 ids 57, 92
        seed(&mut bytes, base + 0x08, &[85, 0, 57, 92]);
        // PP bytes (one per move slot):
        //   slot0 0xB0 -> pp 0x30=48, pp_ups (0xB0>>6)&3 = 2
        //   slot1 0xFF -> ignored (empty move)
        //   slot2 0x0F -> pp 15, pp_ups 0
        //   slot3 0x25 -> pp 0x25=37, pp_ups 0
        seed(&mut bytes, base + 0x1D, &[0xB0, 0xFF, 0x0F, 0x25]);
        seed(&mut bytes, base + 0x21, &[100]); // party LEVEL = 100 (the real one)
        seed(&mut bytes, base + 0x22, &[0x01, 0x00]); // max HP = 256 (BE)
        seed(&mut bytes, base + 0x24, &[0x00, 0xA0]); // atk = 160
        seed(&mut bytes, base + 0x26, &[0x00, 0x96]); // def = 150
        seed(&mut bytes, base + 0x28, &[0x00, 0x8C]); // spd = 140
        seed(&mut bytes, base + 0x2A, &[0x00, 0xC8]); // spc = 200

        SaveData::new(bytes)
    }

    #[test]
    fn parses_scalar_fields() {
        let mon = parse_pokemon(&save_with_mon(), 0);
        assert_eq!(mon.species_id, 131);
        assert_eq!(mon.hp, 255);
        assert_eq!(mon.max_hp, 256);
        assert_eq!(mon.atk, 160);
        assert_eq!(mon.def, 150);
        assert_eq!(mon.spd, 140);
        assert_eq!(mon.spc, 200);
    }

    #[test]
    fn reads_level_from_0x21_not_0x03() {
        // 0x03 holds the decoy value 77; the true party level at 0x21 is 100.
        let mon = parse_pokemon(&save_with_mon(), 0);
        assert_eq!(mon.level, 100);
    }

    #[test]
    fn parses_moves_skipping_empty_slot_and_decoding_pp_ups() {
        let mon = parse_pokemon(&save_with_mon(), 0);
        assert_eq!(
            mon.moves,
            vec![
                MoveSlot { move_id: 85, pp: 48, pp_ups: 2, slot: 0 },
                MoveSlot { move_id: 57, pp: 15, pp_ups: 0, slot: 2 },
                MoveSlot { move_id: 92, pp: 37, pp_ups: 0, slot: 3 },
            ]
        );
    }

    #[test]
    fn parses_paralyze_status() {
        let mon = parse_pokemon(&save_with_mon(), 0);
        assert_eq!(
            mon.status,
            Status {
                sleep_turns: 0,
                poison: false,
                burn: false,
                freeze: false,
                paralyze: true,
            }
        );
    }

    #[test]
    fn not_fainted_when_hp_nonzero() {
        let mon = parse_pokemon(&save_with_mon(), 0);
        assert!(!mon.fainted());
    }

    #[test]
    fn fainted_when_hp_zero() {
        let mut bytes = blank_sram();
        // species + level set, but current HP left at 0.
        seed(&mut bytes, sram::PARTY_DATA, &[19]); // +0x00
        seed(&mut bytes, sram::PARTY_DATA + 0x21, &[50]);
        let mon = parse_pokemon(&SaveData::new(bytes), 0);
        assert_eq!(mon.hp, 0);
        assert!(mon.fainted());
    }

    #[test]
    fn status_any_false_when_clean() {
        let status = Status {
            sleep_turns: 0,
            poison: false,
            burn: false,
            freeze: false,
            paralyze: false,
        };
        assert!(!status.any());
    }

    #[test]
    fn status_any_true_when_paralyzed() {
        let status = Status {
            sleep_turns: 0,
            poison: false,
            burn: false,
            freeze: false,
            paralyze: true,
        };
        assert!(status.any());
    }

    #[test]
    fn sleep_turns_decoded_from_low_three_bits() {
        let mut bytes = blank_sram();
        seed(&mut bytes, sram::PARTY_DATA + 0x01, &[0x00, 0x14]); // hp 20 (not fainted)
        seed(&mut bytes, sram::PARTY_DATA + 0x04, &[0x03]); // status = SLEEP(3)
        let mon = parse_pokemon(&SaveData::new(bytes), 0);
        assert_eq!(mon.status.sleep_turns, 3);
        assert!(!mon.status.poison);
        assert!(!mon.status.paralyze);
    }
}
