//! Shared `#[cfg(test)]` fixtures for the sync crate's tests.
//!
//! Reused across sync tasks (validation, status.json, process pipeline,
//! regression) that need a real, checksum-valid save — often with a specific
//! playtime.

use pokegen1::core::checksum::compute_checksum;
use pokegen1::core::sram::{self, SaveData};

/// Build a minimal VALID Gen-1 save (parses AND passes the checksum) with the
/// given playtime. Used across sync tests that need a real, checksum-valid save.
pub(crate) fn valid_save_bytes(hours: u8, minutes: u8) -> Vec<u8> {
    let mut b = vec![0u8; sram::SAVE_LEN];
    // Minimal parseable party: count 1, one species (131 = MEWTWO). `parse_save`
    // only gates on party count 1..=6, so this is the true minimum.
    b[sram::PARTY_COUNT] = 1;
    b[sram::PARTY_DATA] = 131; // species id at struct +0x00
    b[sram::PLAYTIME_HOURS] = hours;
    b[sram::PLAYTIME_MINUTES] = minutes;
    // Stamp the main-data checksum LAST (party/playtime are inside its range).
    // The checksum byte (0x3523) is outside the summed range, so computing over a
    // clone with it still zero yields the correct value.
    b[sram::MAIN_CHECKSUM] = compute_checksum(&SaveData::new(b.clone()));
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
