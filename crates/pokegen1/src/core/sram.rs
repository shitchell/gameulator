//! SRAM byte accessor and the centralized Gen-1 offset map.
//!
//! Gen-1 save files (`.sav`/`.srm`) are raw SRAM dumps. [`SaveData`] is the
//! typed byte-accessor every later parser reads through (party, items, name,
//! checksum), and this module is the single home for every SRAM offset.
//!
//! Offsets are ported from `reference/read_save.py` and `reference/read_party.py`
//! (Gen-1 SRAM bank 1). Centralizing them is deliberate: when a ROM-version bump
//! moves an offset, there is exactly one file to edit.

// ---- Gen-1 save layout (SRAM bank 1) ----
// One conceptual change (a moved SRAM address) = one contiguous edit block.
// Every offset below has exactly one home here.

/// Trainer name (11 bytes).
pub const NAME: usize = 0x2598;
/// Bag item list (item,qty pairs, `0xFF`-terminated).
pub const BAG_ITEMS: usize = 0x25CA;
/// PC item list (leading count byte, then item,qty pairs, `0xFF`-terminated).
pub const PC_ITEMS: usize = 0x2834;
/// Start of the PC item `(item,qty)` pairs. Immediately follows the [`PC_ITEMS`]
/// count byte; given its own named home per the single-offset-map principle (so
/// callers never open-code `PC_ITEMS + 1`).
pub const PC_ITEMS_DATA: usize = 0x2835;
/// Playtime hours (1 byte).
pub const PLAYTIME_HOURS: usize = 0x2CED;
/// Playtime minutes (1 byte). Note the gap: `0x2CEE` (between hours and
/// minutes) is the max-time-reached flag and is intentionally skipped — minutes
/// is NOT at `PLAYTIME_HOURS + 1`. (Gen-1 playtime block: hours / flag / minutes
/// / frames.)
pub const PLAYTIME_MINUTES: usize = 0x2CEF;
/// Number of Pokémon in the party (1 byte).
pub const PARTY_COUNT: usize = 0x2F2C;
/// Party species-id list (`count` bytes, then a `0xFF` terminator).
/// Immediately follows [`PARTY_COUNT`]; given its own named home per the
/// single-offset-map principle.
pub const PARTY_SPECIES: usize = 0x2F2D;
/// Start of the 6 x 44-byte party structs.
pub const PARTY_DATA: usize = 0x2F34;
/// Original-trainer names, 11 bytes each.
pub const OT_NAMES: usize = 0x2F9C;
/// Party nicknames, 11 bytes each.
pub const NICKNAMES: usize = 0x307E;
/// Length of a single party struct.
pub const PARTY_STRUCT_LEN: usize = 44;
/// Fixed width of name/nickname/OT fields.
pub const NAME_LEN: usize = 11;

// ---- Main-data checksum (Yellow Legacy V1.0.10, disassembly-confirmed) ----
// `SAVCheckSum` (engine/menus/save.asm) sums the bytes of `sGameData` and
// stores the ones-complement at `sMainDataCheckSum`. The built symbol file
// (pokeyellow.sym) pins `sGameData`=01:a598 and `sGameDataEnd`=`sMainDataCheckSum`=01:b523.
// Bank-1 .sav offset = 0x2000 + (addr - 0xa000), giving the file offsets below.

/// First byte covered by the main-data checksum (= [`NAME`] / `sGameData`).
pub const MAIN_DATA_START: usize = 0x2598;
/// Last byte (inclusive) covered by the main-data checksum. The disassembly
/// sums the half-open range `[sGameData, sGameDataEnd)`; `sGameDataEnd`
/// (`0x3523`) is the checksum byte itself, so the summed range ends at `0x3522`.
pub const MAIN_DATA_END: usize = 0x3522;
/// Offset of the stored main-data checksum byte (`sMainDataCheckSum`),
/// immediately after [`MAIN_DATA_END`].
pub const MAIN_CHECKSUM: usize = 0x3523;

/// Typed byte-accessor over a raw Gen-1 SRAM save buffer.
///
/// Accessors index directly into the buffer. Precondition: callers pass
/// in-range offsets drawn from this module's offset map, applied to a full
/// save buffer. Construction-time length validation belongs to `parse_save`
/// (a later task), not here.
pub struct SaveData {
    bytes: Vec<u8>,
}

impl SaveData {
    /// Wrap raw save bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    /// Read a single byte at `offset`.
    pub fn read_u8(&self, offset: usize) -> u8 {
        self.bytes[offset]
    }

    /// Read a big-endian `u16` at `offset` (high byte first, as Gen-1 stores
    /// multi-byte stats/HP).
    pub fn read_u16_be(&self, offset: usize) -> u16 {
        ((self.bytes[offset] as u16) << 8) | (self.bytes[offset + 1] as u16)
    }

    /// Borrow `len` bytes starting at `offset`.
    pub fn slice(&self, offset: usize, len: usize) -> &[u8] {
        &self.bytes[offset..offset + len]
    }

    /// Total length of the save buffer.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Whether the save buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_u16_be_is_big_endian() {
        let mut bytes = vec![0x00u8; 16];
        bytes[4] = 0x12;
        bytes[5] = 0x34;
        let save = SaveData::new(bytes);
        assert_eq!(save.read_u16_be(4), 0x1234);
    }

    #[test]
    fn read_u8_returns_byte_at_offset() {
        let mut bytes = vec![0x00u8; 8];
        bytes[3] = 0xAB;
        let save = SaveData::new(bytes);
        assert_eq!(save.read_u8(3), 0xAB);
    }

    #[test]
    fn slice_returns_subslice() {
        let save = SaveData::new(vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(save.slice(2, 3), &[2, 3, 4]);
    }

    #[test]
    fn len_and_is_empty() {
        let save = SaveData::new(vec![0u8; 5]);
        assert_eq!(save.len(), 5);
        assert!(!save.is_empty());

        let empty = SaveData::new(vec![]);
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn offset_consts_match_reference() {
        assert_eq!(PARTY_COUNT, 0x2F2C);
        assert_eq!(PARTY_STRUCT_LEN, 44);
        assert_eq!(NAME, 0x2598);
        assert_eq!(PARTY_DATA, 0x2F34);
    }
}
