//! Presentation helpers: format the app DTOs into read_save.py-style text.
//!
//! This module is the VIEW. It does NO name resolution, nickname suppression,
//! or fainted logic — those live in `app`. It only maps the already-resolved
//! DTOs to strings. The label-mapping fn ([`status_labels`]) and the party
//! line formatters are the high-value unit-tested pieces.

use app::{Condition, ItemView, PartyMemberView, SaveInfoView};

/// Map a single [`Condition`] to its read_save.py-style label.
fn condition_label(cond: &Condition) -> String {
    match cond {
        Condition::Sleep { turns } => format!("SLEEP({turns})"),
        Condition::Poison => "POISON".to_string(),
        Condition::Burn => "BURN".to_string(),
        Condition::Freeze => "FREEZE".to_string(),
        Condition::Paralyze => "PARALYZE".to_string(),
    }
}

/// Build the comma-joined status label string for a party member.
///
/// A fainted member renders as just `"FAINTED"`, discarding any status bits —
/// matching read_save.py (`if cur==0: conds=["FAINTED"]`), and reflecting that a
/// fainted mon's stored status is not meaningful. Otherwise maps each
/// [`Condition`] to a label. Returns an empty string when healthy and un-fainted.
pub fn status_labels(member: &PartyMemberView) -> String {
    if member.fainted {
        return "FAINTED".to_string();
    }
    member
        .status
        .iter()
        .map(condition_label)
        .collect::<Vec<_>>()
        .join(",")
}

/// The species/nickname display string: `"NICK (SPECIES)"` if a (distinct)
/// nickname is present, else just the species. (The DTO already did suppression.)
fn display_name(member: &PartyMemberView) -> String {
    match &member.nickname {
        Some(nick) => format!("{nick} ({})", member.species),
        None => member.species.clone(),
    }
}

/// The `"  [LABELS]"` status tag appended to a header line, or empty.
fn status_tag(member: &PartyMemberView) -> String {
    let labels = status_labels(member);
    if labels.is_empty() {
        String::new()
    } else {
        format!("  [{labels}]")
    }
}

/// Render the default (verbose) block for one party member: a header line, a
/// stats line, and a moves line.
pub fn party_member_block(member: &PartyMemberView) -> String {
    // slot is 0-based in the DTO; display 1-based (read_save.py-style).
    let header = format!(
        "{}. {} Lv{}  HP {}/{}{}",
        member.slot + 1,
        display_name(member),
        member.level,
        member.hp,
        member.max_hp,
        status_tag(member),
    );
    let stats = format!(
        "   Atk {}  Def {}  Spd {}  Spc {}",
        member.atk, member.def, member.spd, member.spc
    );
    let moves = member
        .moves
        .iter()
        // Only current PP is known (no max PP in the DTO); do not fabricate a max.
        .map(|m| format!("{} ({})", m.name, m.pp))
        .collect::<Vec<_>>()
        .join(" / ");
    format!("{header}\n{stats}\n   {moves}")
}

/// Render a compact, diff-friendly one-liner for a party member.
pub fn party_member_compact(member: &PartyMemberView) -> String {
    let moves = member
        .moves
        .iter()
        .map(|m| format!("{} ({})", m.name, m.pp))
        .collect::<Vec<_>>()
        .join(" \u{b7} ");
    format!(
        "{:10} L{} {}/{} ATK{} DEF{} SPD{} SPC{} {}  {}",
        member.species,
        member.level,
        member.hp,
        member.max_hp,
        member.atk,
        member.def,
        member.spd,
        member.spc,
        status_labels(member),
        moves,
    )
}

/// Render the full party (default view): a count header, then blocks separated
/// by blank lines. (Trainer/playtime live in the `info` subcommand — the CLI
/// splits read_save.py's monolithic dump into party/bag/pc/info.)
pub fn party(members: &[PartyMemberView]) -> String {
    let header = format!("=== PARTY ({}) ===", members.len());
    let blocks = members
        .iter()
        .map(party_member_block)
        .collect::<Vec<_>>()
        .join("\n\n");
    if blocks.is_empty() {
        header
    } else {
        format!("{header}\n{blocks}")
    }
}

/// Render the full party in compact mode: one line per member.
pub fn party_compact(members: &[PartyMemberView]) -> String {
    members
        .iter()
        .map(party_member_compact)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render an item list (bag or PC) with a titled header.
pub fn items(title: &str, items: &[ItemView]) -> String {
    let mut out = format!("=== {title} ({}) ===", items.len());
    for item in items {
        out.push_str(&format!("\n  {:22} x{}", item.name, item.quantity));
    }
    out
}

/// Render save-level info: trainer, playtime, and checksum status.
///
/// Badges are NOT parsed into `Save` yet, so they are omitted here.
pub fn info(info: &SaveInfoView) -> String {
    let checksum = if info.checksum_ok { "OK" } else { "BAD" };
    format!(
        "Trainer: {}\nPlaytime: {}h {}m\nChecksum: {}",
        info.trainer, info.playtime.hours, info.playtime.minutes, checksum
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use app::{Condition, MoveView, PartyMemberView};

    fn member() -> PartyMemberView {
        PartyMemberView {
            slot: 0,
            species: "MEWTWO".to_string(),
            nickname: None,
            level: 70,
            hp: 100,
            max_hp: 180,
            fainted: false,
            atk: 154,
            def: 90,
            spd: 130,
            spc: 194,
            status: vec![],
            moves: vec![],
        }
    }

    // ---- status_labels: all 5 conditions + FAINTED ----

    #[test]
    fn label_sleep_carries_turn_count() {
        let mut m = member();
        m.status = vec![Condition::Sleep { turns: 3 }];
        assert_eq!(status_labels(&m), "SLEEP(3)");
    }

    #[test]
    fn label_poison() {
        let mut m = member();
        m.status = vec![Condition::Poison];
        assert_eq!(status_labels(&m), "POISON");
    }

    #[test]
    fn label_burn() {
        let mut m = member();
        m.status = vec![Condition::Burn];
        assert_eq!(status_labels(&m), "BURN");
    }

    #[test]
    fn label_freeze() {
        let mut m = member();
        m.status = vec![Condition::Freeze];
        assert_eq!(status_labels(&m), "FREEZE");
    }

    #[test]
    fn label_paralyze() {
        let mut m = member();
        m.status = vec![Condition::Paralyze];
        assert_eq!(status_labels(&m), "PARALYZE");
    }

    #[test]
    fn label_fainted_folded_in() {
        let mut m = member();
        m.fainted = true;
        assert_eq!(status_labels(&m), "FAINTED");
    }

    #[test]
    fn label_fainted_replaces_conditions() {
        // A fainted mon shows only FAINTED, discarding status bits (read_save.py
        // parity: `if cur==0: conds=["FAINTED"]`).
        let mut m = member();
        m.status = vec![Condition::Poison, Condition::Burn];
        m.fainted = true;
        assert_eq!(status_labels(&m), "FAINTED");
    }

    #[test]
    fn label_multiple_conditions_joined_with_comma() {
        let mut m = member();
        m.status = vec![Condition::Poison, Condition::Burn];
        assert_eq!(status_labels(&m), "POISON,BURN");
    }

    #[test]
    fn label_healthy_is_empty() {
        assert_eq!(status_labels(&member()), "");
    }

    // ---- header / block formatting ----

    #[test]
    fn block_header_uses_species_when_no_nickname() {
        let block = party_member_block(&member());
        assert!(
            block.starts_with("1. MEWTWO Lv70  HP 100/180"),
            "got: {block}"
        );
    }

    #[test]
    fn block_header_shows_nickname_and_species() {
        let mut m = member();
        m.nickname = Some("SPARKY".to_string());
        m.species = "PIKACHU".to_string();
        let block = party_member_block(&m);
        assert!(
            block.starts_with("1. SPARKY (PIKACHU) Lv70"),
            "got: {block}"
        );
    }

    #[test]
    fn block_header_appends_status_tag() {
        let mut m = member();
        m.status = vec![Condition::Paralyze];
        let block = party_member_block(&m);
        let first = block.lines().next().unwrap();
        assert!(first.ends_with("  [PARALYZE]"), "got: {first}");
    }

    #[test]
    fn block_header_no_tag_when_healthy() {
        let block = party_member_block(&member());
        let first = block.lines().next().unwrap();
        assert!(!first.contains('['), "got: {first}");
    }

    #[test]
    fn block_stats_line() {
        let block = party_member_block(&member());
        let stats = block.lines().nth(1).unwrap();
        assert_eq!(stats, "   Atk 154  Def 90  Spd 130  Spc 194");
    }

    #[test]
    fn block_moves_render_current_pp_only() {
        let mut m = member();
        m.moves = vec![
            MoveView { name: "THUNDERBOLT".to_string(), pp: 15, pp_ups: 3, slot: 0 },
            MoveView { name: "PSYCHIC".to_string(), pp: 10, pp_ups: 0, slot: 1 },
        ];
        let block = party_member_block(&m);
        let moves = block.lines().nth(2).unwrap();
        assert_eq!(moves, "   THUNDERBOLT (15) / PSYCHIC (10)");
    }

    // ---- compact ----

    #[test]
    fn compact_one_line_with_stats_and_moves() {
        let mut m = member();
        m.species = "MEWTWO".to_string();
        m.moves = vec![
            MoveView { name: "PSYCHIC".to_string(), pp: 10, pp_ups: 0, slot: 0 },
            MoveView { name: "RECOVER".to_string(), pp: 20, pp_ups: 0, slot: 1 },
        ];
        let line = party_member_compact(&m);
        assert_eq!(line.lines().count(), 1);
        assert!(line.contains("MEWTWO"), "got: {line}");
        assert!(line.contains("L70"), "got: {line}");
        assert!(line.contains("100/180"), "got: {line}");
        assert!(line.contains("PSYCHIC (10) \u{b7} RECOVER (20)"), "got: {line}");
    }

    #[test]
    fn compact_includes_status_labels() {
        let mut m = member();
        m.status = vec![Condition::Poison];
        let line = party_member_compact(&m);
        assert!(line.contains("POISON"), "got: {line}");
    }

    // ---- items ----

    #[test]
    fn items_header_and_rows() {
        let out = items(
            "BAG",
            &[
                ItemView { name: "POTION".to_string(), quantity: 5 },
                ItemView { name: "MASTER BALL".to_string(), quantity: 1 },
            ],
        );
        assert!(out.starts_with("=== BAG (2) ==="), "got: {out}");
        assert!(out.contains("  POTION                 x5"), "got: {out}");
        assert!(out.contains("x1"), "got: {out}");
    }

    // ---- info ----

    #[test]
    fn info_renders_checksum_ok() {
        let v = SaveInfoView {
            trainer: "RED".to_string(),
            playtime: pokegen1::Playtime { hours: 24, minutes: 12 },
            checksum_ok: true,
        };
        let out = info(&v);
        assert!(out.contains("Trainer: RED"), "got: {out}");
        assert!(out.contains("Playtime: 24h 12m"), "got: {out}");
        assert!(out.contains("Checksum: OK"), "got: {out}");
    }

    #[test]
    fn info_renders_checksum_bad() {
        let v = SaveInfoView {
            trainer: "RED".to_string(),
            playtime: pokegen1::Playtime { hours: 1, minutes: 2 },
            checksum_ok: false,
        };
        assert!(info(&v).contains("Checksum: BAD"));
    }
}
