//! Party parsing: the count gate and the ordered species-id list.
//!
//! [`party_count`] is the entry gate to all party parsing (later tasks build the
//! per-mon structs on top of it): its `1..=6` validation is the primary
//! "is this actually a Gen-1 party save?" sanity check. [`party_species`] gives
//! the ordered species ids that the per-mon struct parse will corroborate.

use crate::core::error::ParseError;
use crate::core::pokemon::{self, Pokemon};
use crate::core::sram::{self, SaveData};
use crate::core::text;

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

/// Parse the full party into per-mon [`Pokemon`] structs, with nicknames.
///
/// Gates on [`party_count`] (so an invalid count yields
/// [`ParseError::InvalidPartyCount`]), then parses each 44-byte struct via
/// `pokemon::parse_pokemon` and layers on the nickname from the party
/// nickname region (which lives OUTSIDE the struct, at
/// [`sram::NICKNAMES`] `+ i * `[`sram::NAME_LEN`]).
///
/// The nickname is stored raw here (or `None` if empty). The "nickname equals
/// species name ⇒ suppress it" resolution happens later in the app/view layer,
/// which owns the species-name table; core stays table-free.
pub fn parse_party(save: &SaveData) -> Result<Vec<Pokemon>, ParseError> {
    let n = party_count(save)? as usize;
    let mut party = Vec::with_capacity(n);
    for i in 0..n {
        let mut mon = pokemon::parse_pokemon(save, i);
        let raw =
            text::decode_string(save.slice(sram::NICKNAMES + i * sram::NAME_LEN, sram::NAME_LEN));
        mon.nickname = if raw.is_empty() { None } else { Some(raw) };
        party.push(mon);
    }
    Ok(party)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_support::{blank_sram, seed};

    /// Build a synthetic save buffer with a given party count and species ids.
    fn synthetic_save(count: u8, species: &[u8]) -> SaveData {
        let mut bytes = blank_sram();
        seed(&mut bytes, sram::PARTY_COUNT, &[count]);
        seed(&mut bytes, sram::PARTY_SPECIES, species);
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
        assert_eq!(party_species(&save), Err(ParseError::InvalidPartyCount(0)));
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

    /// Seed a 44-byte party struct at `slot` with a species id and level.
    fn seed_mon(bytes: &mut [u8], slot: usize, species: u8, level: u8) {
        let base = sram::PARTY_DATA + slot * sram::PARTY_STRUCT_LEN;
        seed(bytes, base, &[species]); // +0x00 species
        seed(bytes, base + 0x01, &[0x00, 0x14]); // cur HP = 20 (not fainted)
        seed(bytes, base + 0x21, &[level]); // party level at 0x21
    }

    #[test]
    fn parses_two_mon_party_with_nicknames() {
        let mut bytes = blank_sram();
        seed(&mut bytes, sram::PARTY_COUNT, &[2]);
        seed_mon(&mut bytes, 0, 131, 100); // MEWTWO L100
        seed_mon(&mut bytes, 1, 19, 25); // LAPRAS-slot L25 (species just distinct)

        // "SPARKY": S=0x92 P=0x8F A=0x80 R=0x91 K=0x8A Y=0x98 then 0x50
        seed(
            &mut bytes,
            sram::NICKNAMES,
            &[0x92, 0x8F, 0x80, 0x91, 0x8A, 0x98, 0x50],
        );
        // "MEW": M=0x8C E=0x84 W=0x96 then 0x50
        seed(
            &mut bytes,
            sram::NICKNAMES + sram::NAME_LEN,
            &[0x8C, 0x84, 0x96, 0x50],
        );

        let party = parse_party(&SaveData::new(bytes)).unwrap();
        assert_eq!(party.len(), 2);

        assert_eq!(party[0].species_id, 131);
        assert_eq!(party[0].level, 100);
        assert_eq!(party[0].nickname, Some("SPARKY".to_string()));

        assert_eq!(party[1].species_id, 19);
        assert_eq!(party[1].level, 25);
        assert_eq!(party[1].nickname, Some("MEW".to_string()));
    }

    #[test]
    fn empty_nickname_region_yields_none() {
        let mut bytes = blank_sram();
        seed(&mut bytes, sram::PARTY_COUNT, &[1]);
        seed_mon(&mut bytes, 0, 25, 5);
        // Nickname region: immediate terminator -> empty -> None.
        seed(&mut bytes, sram::NICKNAMES, &[0x50]);

        let party = parse_party(&SaveData::new(bytes)).unwrap();
        assert_eq!(party.len(), 1);
        assert_eq!(party[0].nickname, None);
    }

    #[test]
    fn zeroed_nickname_region_yields_none() {
        let mut bytes = blank_sram();
        seed(&mut bytes, sram::PARTY_COUNT, &[1]);
        seed_mon(&mut bytes, 0, 25, 5);
        // Region left as zeros (blank_sram default) -> terminator -> None.

        let party = parse_party(&SaveData::new(bytes)).unwrap();
        assert_eq!(party[0].nickname, None);
    }

    #[test]
    fn parse_party_errors_on_invalid_count() {
        let save = synthetic_save(0, &[]);
        assert_eq!(parse_party(&save), Err(ParseError::InvalidPartyCount(0)));
    }
}
