//! Gen-1 main-data SRAM checksum.
//!
//! Ported from the Yellow Legacy disassembly's `SAVCheckSum` routine
//! (`engine/menus/save.asm`, tag V1.0.10): sum every byte of the main data
//! block, then store the ones-complement of the low byte. The built symbol
//! file pins the summed range and the stored-checksum offset; see
//! [`sram::MAIN_DATA_START`], [`sram::MAIN_DATA_END`], [`sram::MAIN_CHECKSUM`].
//!
//! This guard is load-bearing for the sync watcher: a half-written or corrupt
//! save must be caught before it propagates.

use crate::core::sram::{self, SaveData};

/// Recompute the main-data checksum over `[MAIN_DATA_START, MAIN_DATA_END]`.
///
/// Mirrors `SAVCheckSum`: accumulate all bytes into a wrapping `u8`, then
/// return the ones-complement (`cpl`) of the low byte.
///
/// Exposed `pub` (beyond [`verify_checksum`]'s boolean gate) so the sync watcher
/// can log a computed-vs-stored mismatch, and a future save-writer can recompute
/// the byte to store — both without duplicating this algorithm. (Exposing the
/// computed byte leaks nothing that [`verify_checksum`] doesn't already compare.)
pub fn compute_checksum(save: &SaveData) -> u8 {
    let mut sum: u8 = 0;
    for offset in sram::MAIN_DATA_START..=sram::MAIN_DATA_END {
        sum = sum.wrapping_add(save.read_u8(offset));
    }
    sum ^ 0xFF
}

/// Whether the save's stored main-data checksum matches a fresh recomputation.
pub fn verify_checksum(save: &SaveData) -> bool {
    compute_checksum(save) == save.read_u8(sram::MAIN_CHECKSUM)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_support::blank_sram;

    /// Mirror of the disassembly algorithm, kept independent of the impl so the
    /// test exercises the range/operation directly.
    fn expected_checksum(buf: &[u8]) -> u8 {
        let mut sum: u8 = 0;
        for &b in &buf[sram::MAIN_DATA_START..=sram::MAIN_DATA_END] {
            sum = sum.wrapping_add(b);
        }
        sum ^ 0xFF
    }

    #[test]
    fn verify_checksum_accepts_correct_checksum() {
        let mut buf = blank_sram();
        // Put some non-trivial data in the checksummed range.
        buf[sram::MAIN_DATA_START] = 0x92;
        buf[sram::MAIN_DATA_START + 100] = 0x37;
        buf[sram::MAIN_DATA_END] = 0xAB;
        buf[sram::MAIN_CHECKSUM] = expected_checksum(&buf);
        let save = SaveData::new(buf);
        assert!(verify_checksum(&save));
    }

    #[test]
    fn verify_checksum_detects_corruption_inside_range() {
        let mut buf = blank_sram();
        buf[sram::MAIN_DATA_START] = 0x92;
        buf[sram::MAIN_CHECKSUM] = expected_checksum(&buf);
        // Flip a byte inside the summed range without fixing the checksum.
        buf[sram::MAIN_DATA_START + 50] ^= 0xFF;
        let save = SaveData::new(buf);
        assert!(!verify_checksum(&save));
    }

    #[test]
    fn verify_checksum_ignores_bytes_outside_range() {
        let mut buf = blank_sram();
        buf[sram::MAIN_DATA_START] = 0x92;
        buf[sram::MAIN_CHECKSUM] = expected_checksum(&buf);
        // A byte just past the summed range must not affect validity.
        buf[sram::MAIN_DATA_END + 2] ^= 0xFF;
        let save = SaveData::new(buf);
        assert!(verify_checksum(&save));
    }
}
