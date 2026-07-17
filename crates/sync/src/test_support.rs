//! Shared `#[cfg(test)]` fixtures for the sync crate's tests.
//!
//! pokegen1's `compute_checksum` is `pub(crate)` and unreachable here, so this
//! helper recomputes the main-data checksum itself using pokegen1's PUBLIC
//! `core::sram` offset consts. Reused across sync tasks (validation, status.json,
//! process pipeline, regression) that need a real, checksum-valid save — often
//! with a specific playtime.

use pokegen1::core::sram;

/// Build a minimal VALID Gen-1 save (parses AND passes the checksum) with the
/// given playtime. Used across sync tests that need a real, checksum-valid save.
pub(crate) fn valid_save_bytes(hours: u8, minutes: u8) -> Vec<u8> {
    let mut b = vec![0u8; sram::SAVE_LEN];
    // Minimal parseable party: count 1, one species (131 = MEWTWO) at the struct base.
    b[sram::PARTY_COUNT] = 1;
    b[sram::PARTY_DATA] = 131; // species id at struct +0x00
    b[sram::PARTY_DATA + 0x01] = 0x00; // current HP hi (BE); nonzero below
    b[sram::PARTY_DATA + 0x02] = 0x14; // current HP lo = 20 (not fainted)
    b[sram::PARTY_DATA + 0x21] = 50; // level at +0x21
    b[sram::PLAYTIME_HOURS] = hours;
    b[sram::PLAYTIME_MINUTES] = minutes;
    // Compute the main-data checksum LAST (playtime/party are within its range):
    // sum bytes [MAIN_DATA_START, MAIN_DATA_END] in a wrapping u8, ones-complement.
    let mut sum: u8 = 0;
    for &byte in &b[sram::MAIN_DATA_START..=sram::MAIN_DATA_END] {
        sum = sum.wrapping_add(byte);
    }
    b[sram::MAIN_CHECKSUM] = sum ^ 0xFF;
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Proves the helper really builds a checksum-valid save: if this fails, the
    /// offsets/algorithm are wrong and must be fixed before other tasks rely on it.
    #[test]
    fn valid_save_bytes_produces_checksum_ok_save() {
        let save = pokegen1::parse_save(valid_save_bytes(1, 2)).unwrap();
        assert!(save.checksum_ok);
    }
}
