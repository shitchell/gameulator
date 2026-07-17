//! Pure parsers over Yellow Legacy disassembly text.
//!
//! Each parser takes raw `.asm` file text and returns a deterministic
//! `BTreeMap<u16, String>` of id -> display name. These are separated from
//! all file I/O so they can be unit-tested against inline fixtures.

use std::collections::BTreeMap;

/// Extract the quoted string payload of a `db "..."` or `li "..."` line.
/// Returns `None` for lines that are not a quoted name entry.
fn quoted_name(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix("db ")
        .or_else(|| trimmed.strip_prefix("li "))?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Strip GameBoy string padding: everything from the first `@` onward is
/// terminator/padding in the fixed-width `db "NAME@@@@"` species table.
fn strip_padding(s: &str) -> String {
    match s.find('@') {
        Some(idx) => s[..idx].to_string(),
        None => s.to_string(),
    }
}

/// Parse `data/pokemon/names.asm` (MonsterNames): fixed-width `db "NAME@@@@"`
/// entries in **internal id order** starting at 1. MISSINGNO. gap entries are
/// present in the table and are kept (they occupy real internal ids).
pub fn parse_species(text: &str) -> BTreeMap<u16, String> {
    let mut out = BTreeMap::new();
    let mut id: u16 = 1;
    for line in text.lines() {
        let t = line.trim_start();
        if !t.starts_with("db \"") {
            continue;
        }
        if let Some(raw) = quoted_name(line) {
            out.insert(id, strip_padding(&raw));
            id += 1;
        }
    }
    out
}

/// Parse a positional `li "NAME"` list (moves / items) starting at id 1.
/// Stops at `stop_marker` if provided (e.g. `assert_list_length NUM_ITEMS`
/// for items, which is followed by unrelated elevator-floor names).
fn parse_li_list(text: &str, stop_marker: Option<&str>) -> BTreeMap<u16, String> {
    let mut out = BTreeMap::new();
    let mut id: u16 = 1;
    for line in text.lines() {
        let t = line.trim();
        if let Some(marker) = stop_marker {
            if t.starts_with(marker) {
                break;
            }
        }
        if t.starts_with("li \"") {
            if let Some(raw) = quoted_name(line) {
                out.insert(id, raw);
                id += 1;
            }
        }
    }
    out
}

/// Parse `data/moves/names.asm` (MoveNames): positional `li "NAME"` list,
/// move id 1..=165.
pub fn parse_moves(text: &str) -> BTreeMap<u16, String> {
    parse_li_list(text, None)
}

/// Parse the "regular" bag items from `data/items/names.asm` (ItemNames):
/// positional `li "NAME"` list starting at id 1, stopping at the
/// `assert_list_length NUM_ITEMS` marker (elevator floor names follow it and
/// are not real items).
pub fn parse_regular_items(text: &str) -> BTreeMap<u16, String> {
    parse_li_list(text, Some("assert_list_length NUM_ITEMS"))
}

/// Title-case a SCREAMING_SNAKE move constant, e.g. `MEGA_PUNCH` -> `Mega Punch`.
/// A trailing `_M` disambiguator (e.g. `PSYCHIC_M`) becomes ` M` to match the
/// read_save.py oracle style.
fn titlecase_move_const(name: &str) -> String {
    name.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Parse HM/TM item names + ids from `constants/item_constants.asm`.
///
/// HMs occupy item ids $C4..$C8 (196..200) in `add_hm` order; TMs occupy
/// $C9..$FA (201..250) in `add_tm` order. Each entry references a move
/// constant; the display name is `HM-<Move>` / `TM-<Move>` (title-cased),
/// matching the read_save.py oracle (e.g. 196 -> "HM-Cut").
pub fn parse_tmhm_items(item_constants: &str) -> BTreeMap<u16, String> {
    const HM_BASE: u16 = 0xC4; // 196
    const TM_BASE: u16 = 0xC9; // 201

    let mut out = BTreeMap::new();

    let mut hm_i: u16 = 0;
    let mut tm_i: u16 = 0;
    for line in item_constants.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("add_hm ") {
            if let Some(mv) = first_ident(rest) {
                out.insert(HM_BASE + hm_i, format!("HM-{}", titlecase_move_const(&mv)));
                hm_i += 1;
            }
        } else if let Some(rest) = t.strip_prefix("add_tm ") {
            if let Some(mv) = first_ident(rest) {
                out.insert(TM_BASE + tm_i, format!("TM-{}", titlecase_move_const(&mv)));
                tm_i += 1;
            }
        }
    }
    out
}

/// First whitespace/comment-delimited identifier token from a string.
fn first_ident(s: &str) -> Option<String> {
    let tok = s.split_whitespace().next()?;
    if tok.starts_with(';') {
        return None;
    }
    Some(tok.to_string())
}

/// Build the full item id -> name map: regular bag items (1..) plus HM/TM
/// entries (196.. / 201..).
pub fn parse_items(item_names: &str, item_constants: &str) -> BTreeMap<u16, String> {
    let mut out = parse_regular_items(item_names);
    out.extend(parse_tmhm_items(item_constants));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn species_keeps_internal_id_ordering_and_gaps() {
        // Mimics data/pokemon/names.asm: header, fixed-width padded names,
        // including MISSINGNO. gap entries that occupy real internal ids.
        let fixture = "\
MonsterNames::
\ttable_width NAME_LENGTH - 1, MonsterNames
\tdb \"RHYDON@@@@\"
\tdb \"KANGASKHAN\"
\tdb \"NIDORAN\u{2642}@@\"
\tdb \"MISSINGNO.\"
\tdb \"GROWLITHE@\"
";
        let m = parse_species(fixture);
        assert_eq!(m.get(&1).map(String::as_str), Some("RHYDON"));
        assert_eq!(m.get(&2).map(String::as_str), Some("KANGASKHAN"));
        assert_eq!(m.get(&3).map(String::as_str), Some("NIDORAN\u{2642}"));
        // gap entry occupies id 4 (kept, not skipped)
        assert_eq!(m.get(&4).map(String::as_str), Some("MISSINGNO."));
        assert_eq!(m.get(&5).map(String::as_str), Some("GROWLITHE"));
        assert_eq!(m.len(), 5);
    }

    #[test]
    fn moves_are_positional_from_one() {
        let fixture = "\
MoveNames::
\tlist_start MoveNames
\tli \"POUND\"
\tli \"KARATE CHOP\"
\tli \"DOUBLESLAP\"
";
        let m = parse_moves(fixture);
        assert_eq!(m.get(&1).map(String::as_str), Some("POUND"));
        assert_eq!(m.get(&2).map(String::as_str), Some("KARATE CHOP"));
        assert_eq!(m.get(&3).map(String::as_str), Some("DOUBLESLAP"));
        assert_eq!(m.len(), 3);
    }

    #[test]
    fn regular_items_stop_at_num_items_marker() {
        // Floor names follow the assert and must NOT be captured as items.
        let fixture = "\
ItemNames::
\tlist_start ItemNames
\tli \"MASTER BALL\"
\tli \"ULTRA BALL\"
\tli \"POK\u{e9} BALL\"
\tassert_list_length NUM_ITEMS
\tli \"B2F\"
\tli \"B1F\"
";
        let m = parse_regular_items(fixture);
        assert_eq!(m.get(&1).map(String::as_str), Some("MASTER BALL"));
        assert_eq!(m.get(&2).map(String::as_str), Some("ULTRA BALL"));
        assert_eq!(m.get(&3).map(String::as_str), Some("POK\u{e9} BALL"));
        assert_eq!(m.len(), 3, "floor names after the assert must be excluded");
    }

    #[test]
    fn tmhm_items_ids_and_titlecasing() {
        let fixture = "\
DEF HM01 EQU const_value
\tadd_hm CUT          ; $C4
\tadd_hm FLY          ; $C5
DEF NUM_HMS EQU const_value - HM01
DEF TM01 EQU const_value
\tadd_tm MEGA_PUNCH   ; $C9
\tadd_tm RAZOR_WIND   ; $CA
\tadd_tm PSYCHIC_M    ; $E5
";
        let m = parse_tmhm_items(fixture);
        assert_eq!(m.get(&196).map(String::as_str), Some("HM-Cut"));
        assert_eq!(m.get(&197).map(String::as_str), Some("HM-Fly"));
        assert_eq!(m.get(&201).map(String::as_str), Some("TM-Mega Punch"));
        assert_eq!(m.get(&202).map(String::as_str), Some("TM-Razor Wind"));
        assert_eq!(m.get(&203).map(String::as_str), Some("TM-Psychic M"));
    }
}
