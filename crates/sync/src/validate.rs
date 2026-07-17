//! The watcher's quarantine gate: whether a save is fully valid.

/// Whether `bytes` is a fully-valid Gen-1 save: it parses AND its stored checksum
/// matches. This is the watcher's quarantine gate — a mid-write or corrupt file
/// (parse error, or Ok-but-checksum_ok==false) does NOT pass and is skipped.
pub fn is_valid_save(bytes: &[u8]) -> bool {
    match pokegen1::parse_save(bytes.to_vec()) {
        Ok(save) => save.checksum_ok,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::valid_save_bytes;
    use pokegen1::core::sram;

    #[test]
    fn accepts_a_checksum_valid_save() {
        assert!(is_valid_save(&valid_save_bytes(10, 30)));
    }

    #[test]
    fn rejects_in_range_corruption_that_breaks_the_checksum() {
        let mut bytes = valid_save_bytes(10, 30);
        // Flip a byte INSIDE the checksummed range WITHOUT recomputing the checksum.
        bytes[sram::MAIN_DATA_START + 100] ^= 0xFF;
        assert!(!is_valid_save(&bytes));
    }

    #[test]
    fn rejects_too_short_buffer_without_panic() {
        // parse_save returns Err(TruncatedSave); the gate must not panic.
        assert!(!is_valid_save(&[0u8; 100]));
    }

    #[test]
    fn rejects_full_length_buffer_with_zero_party_count() {
        // parse_save returns Err(InvalidPartyCount).
        assert!(!is_valid_save(&[0u8; sram::SAVE_LEN]));
    }
}
