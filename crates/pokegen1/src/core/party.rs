//! Party parsing: the count gate and the ordered species-id list.
//!
//! [`party_count`] is the entry gate to all party parsing (later tasks build the
//! per-mon structs on top of it): its `1..=6` validation is the primary
//! "is this actually a Gen-1 party save?" sanity check. [`party_species`] gives
//! the ordered species ids that the per-mon struct parse will corroborate.

use crate::core::error::ParseError;
use crate::core::sram::{self, SaveData};

/// Read and validate the party count.
///
/// Reads the byte at [`sram::PARTY_COUNT`]. Returns `Ok(n)` when `n` is in
/// `1..=6`, else [`ParseError::InvalidPartyCount`]. Mirrors the reference
/// scripts' `1 <= n <= 6` guard.
pub fn party_count(save: &SaveData) -> Result<u8, ParseError> {
    let n = save.read_u8(sram::PARTY_COUNT);
    match n {
        1..=6 => Ok(n),
        _ => Err(ParseError::InvalidPartyCount(n)),
    }
}

/// Read the ordered list of party species ids.
///
/// Returns exactly `party_count` species-id bytes starting at
/// [`sram::PARTY_SPECIES`]. The in-game list is `count` bytes then a `0xFF`
/// terminator, but since the (validated) count is already known we read exactly
/// `count` bytes rather than scanning for the terminator.
pub fn party_species(save: &SaveData) -> Result<Vec<u8>, ParseError> {
    let count = party_count(save)? as usize;
    Ok(save.slice(sram::PARTY_SPECIES, count).to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic save buffer with a given party count and species ids.
    fn synthetic_save(count: u8, species: &[u8]) -> SaveData {
        let mut bytes = vec![0u8; 0x8000];
        bytes[sram::PARTY_COUNT] = count;
        for (i, &sp) in species.iter().enumerate() {
            bytes[sram::PARTY_SPECIES + i] = sp;
        }
        SaveData::new(bytes)
    }

    #[test]
    fn party_count_and_species_are_read() {
        // 131 = MEWTWO, 19 = LAPRAS
        let save = synthetic_save(2, &[131, 19]);
        assert_eq!(party_count(&save), Ok(2));
        assert_eq!(party_species(&save), Ok(vec![131, 19]));
    }

    #[test]
    fn party_species_length_matches_count() {
        let save = synthetic_save(3, &[1, 2, 3, 4, 5]);
        assert_eq!(party_species(&save), Ok(vec![1, 2, 3]));
    }

    #[test]
    fn party_count_zero_is_invalid() {
        let save = synthetic_save(0, &[]);
        assert_eq!(party_count(&save), Err(ParseError::InvalidPartyCount(0)));
        assert_eq!(
            party_species(&save),
            Err(ParseError::InvalidPartyCount(0))
        );
    }

    #[test]
    fn party_count_seven_is_invalid() {
        let save = synthetic_save(7, &[]);
        assert_eq!(party_count(&save), Err(ParseError::InvalidPartyCount(7)));
    }

    #[test]
    fn party_count_boundaries_one_and_six_are_ok() {
        assert_eq!(party_count(&synthetic_save(1, &[42])), Ok(1));
        assert_eq!(party_count(&synthetic_save(6, &[1, 2, 3, 4, 5, 6])), Ok(6));
    }
}
