//! Controller layer — presentation-agnostic operations over a parsed Save +
//! game data, returning serde view DTOs consumed by the cli and web views.

use std::path::Path;

use anyhow::Context;
use serde::Serialize;

// Re-export the pokegen1 types this crate's PUBLIC API references, so callers
// (cli, sync, web) depend only on `app` and never reach into pokegen1 to name a
// type an `app` signature requires.
pub use pokegen1::{GameData, ItemStack, Playtime, Save};
use pokegen1::{Pokemon, Status};

/// The set of supported games. Add variants as overlays land (e.g. `Blue`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameId {
    YellowLegacy,
}

/// Map a [`GameId`] to its concrete overlay, boxed as a `dyn GameData`.
///
/// The controller owns the id→overlay mapping so views stay game-agnostic:
/// a view passes a [`GameId`] and receives a resolver, never naming an overlay
/// crate. Adding a game = one new arm here, with zero view changes.
pub fn game_data(game: GameId) -> Box<dyn GameData> {
    match game {
        GameId::YellowLegacy => Box::new(pokegen1::YellowLegacy::new()),
    }
}

/// A single status condition on a party member. Views format these however they
/// like (the CLI renders read_save.py-style labels; a web view can badge them).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Condition {
    Sleep { turns: u8 },
    Poison,
    Burn,
    Freeze,
    Paralyze,
}

/// A single party member, resolved and materialized for presentation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PartyMemberView {
    pub slot: u8,
    /// Resolved species name. Unresolved ids render as `#<id>`.
    pub species: String,
    pub nickname: Option<String>,
    pub level: u8,
    pub hp: u16,
    pub max_hp: u16,
    pub fainted: bool,
    pub atk: u16,
    pub def: u16,
    pub spd: u16,
    pub spc: u16,
    /// Structured status conditions. Rendering the label strings (e.g.
    /// `SLEEP(3)`, `POISON`) belongs to the view, not this controller.
    pub status: Vec<Condition>,
    pub moves: Vec<MoveView>,
}

/// A single move slot, name-resolved for presentation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MoveView {
    /// Resolved move name. Unresolved ids render as `#<id>`.
    pub name: String,
    pub pp: u8,
    pub pp_ups: u8,
    pub slot: u8,
}

/// A single item stack, name-resolved for presentation (bag or PC).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ItemView {
    /// Resolved item name. Unresolved ids render as `#<id>`.
    pub name: String,
    pub quantity: u8,
}

/// Save-level metadata for presentation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SaveInfoView {
    pub trainer: String,
    pub playtime: Playtime,
    pub checksum_ok: bool,
}

/// Resolve an id to a display name, falling back to `#<id>` for unknown ids
/// (mirrors read_save.py's `#id`). Single home for the fallback convention.
fn resolve(name: Option<&str>, id: u8) -> String {
    name.map(str::to_string).unwrap_or_else(|| format!("#{id}"))
}

/// Build the structured status conditions from a [`Status`] bitfield.
///
/// Mirrors `read_save.py`'s condition list and order: Sleep (only when
/// `sleep_turns > 0`, carrying the turn count), then Poison, Burn, Freeze,
/// Paralyze. Fainted is deliberately NOT included here — it is the separate
/// `fainted` bool on [`PartyMemberView`], materialized from
/// `Pokemon::fainted()`. A healthy mon yields an empty `Vec`. Views own the
/// mapping from [`Condition`] to label strings.
fn conditions(status: &Status) -> Vec<Condition> {
    let mut out = Vec::new();
    if status.sleep_turns > 0 {
        out.push(Condition::Sleep {
            turns: status.sleep_turns,
        });
    }
    if status.poison {
        out.push(Condition::Poison);
    }
    if status.burn {
        out.push(Condition::Burn);
    }
    if status.freeze {
        out.push(Condition::Freeze);
    }
    if status.paralyze {
        out.push(Condition::Paralyze);
    }
    out
}

/// Load and parse a Gen-1 save from `path`.
pub fn load_save(path: &Path) -> anyhow::Result<Save> {
    let bytes =
        std::fs::read(path).with_context(|| format!("reading save file {}", path.display()))?;
    pokegen1::parse_save(bytes).with_context(|| format!("parsing save file {}", path.display()))
}

/// Build a presentation-ready summary of the party.
pub fn party_summary(save: &Save, game: &dyn GameData) -> Vec<PartyMemberView> {
    save.party
        .iter()
        .enumerate()
        .map(|(i, mon)| party_member_view(i as u8, mon, game))
        .collect()
}

/// Build a presentation-ready view of an item list (bag or PC).
///
/// Called for both `&save.bag` and `&save.pc`.
pub fn items_view(items: &[ItemStack], game: &dyn GameData) -> Vec<ItemView> {
    items
        .iter()
        .map(|item| ItemView {
            name: resolve(game.item_name(item.item_id), item.item_id),
            quantity: item.quantity,
        })
        .collect()
}

/// Build save-level metadata for presentation.
pub fn save_info(save: &Save) -> SaveInfoView {
    SaveInfoView {
        trainer: save.trainer.clone(),
        playtime: save.playtime.clone(),
        checksum_ok: save.checksum_ok,
    }
}

/// Resolve a single [`Pokemon`] into its [`PartyMemberView`].
///
/// Owns the three deferred presentation rules: species-name resolution with a
/// `#id` fallback, nickname suppression (keep it only if it differs from the
/// resolved species name), and materialization of the `fainted` bool.
fn party_member_view(slot: u8, mon: &Pokemon, game: &dyn GameData) -> PartyMemberView {
    let species = resolve(game.species_name(mon.species_id), mon.species_id);

    // Nickname suppression: keep only a nickname that differs from the resolved
    // species name (mirrors read_save.py's `nick if nick and nick!=nm else None`).
    let nickname = mon
        .nickname
        .as_ref()
        .filter(|nick| nick.as_str() != species)
        .cloned();

    let moves = mon
        .moves
        .iter()
        .map(|m| MoveView {
            name: resolve(game.move_name(m.move_id), m.move_id),
            pp: m.pp,
            pp_ups: m.pp_ups,
            slot: m.slot,
        })
        .collect();

    PartyMemberView {
        slot,
        species,
        nickname,
        level: mon.level,
        hp: mon.hp,
        max_hp: mon.max_hp,
        fainted: mon.fainted(),
        atk: mon.atk,
        def: mon.def,
        spd: mon.spd,
        spc: mon.spc,
        status: conditions(&mon.status),
        moves,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pokegen1::{ItemTable, MoveSlot, MoveTable, SpeciesTable};

    /// Tiny stub `GameData` so view tests don't couple to the real overlay.
    struct StubData;

    impl SpeciesTable for StubData {
        fn species_name(&self, id: u8) -> Option<&str> {
            match id {
                131 => Some("MEWTWO"),
                25 => Some("PIKACHU"),
                _ => None,
            }
        }
    }
    impl MoveTable for StubData {
        fn move_name(&self, id: u8) -> Option<&str> {
            match id {
                85 => Some("THUNDERBOLT"),
                _ => None,
            }
        }
    }
    impl ItemTable for StubData {
        fn item_name(&self, id: u8) -> Option<&str> {
            match id {
                1 => Some("MASTER BALL"),
                _ => None,
            }
        }
    }

    fn clean_status() -> Status {
        Status {
            sleep_turns: 0,
            poison: false,
            burn: false,
            freeze: false,
            paralyze: false,
        }
    }

    fn mon(species_id: u8, nickname: Option<&str>) -> Pokemon {
        Pokemon {
            species_id,
            level: 50,
            hp: 100,
            max_hp: 100,
            atk: 60,
            def: 55,
            spd: 70,
            spc: 65,
            moves: vec![],
            status: clean_status(),
            nickname: nickname.map(|s| s.to_string()),
        }
    }

    fn save_with_party(party: Vec<Pokemon>) -> Save {
        Save {
            trainer: "RED".to_string(),
            playtime: Playtime {
                hours: 24,
                minutes: 12,
            },
            party,
            bag: vec![],
            pc: vec![],
            checksum_ok: true,
        }
    }

    #[test]
    fn party_summary_resolves_species_name() {
        let save = save_with_party(vec![mon(131, None)]);
        let out = party_summary(&save, &StubData);
        assert_eq!(out[0].species, "MEWTWO");
        assert_eq!(out[0].slot, 0);
    }

    #[test]
    fn party_summary_unknown_species_falls_back_to_hash_id() {
        let save = save_with_party(vec![mon(200, None)]);
        let out = party_summary(&save, &StubData);
        assert_eq!(out[0].species, "#200");
    }

    #[test]
    fn party_summary_suppresses_nickname_equal_to_species() {
        let save = save_with_party(vec![mon(25, Some("PIKACHU"))]);
        let out = party_summary(&save, &StubData);
        assert_eq!(out[0].nickname, None);
    }

    #[test]
    fn party_summary_keeps_distinct_nickname() {
        let save = save_with_party(vec![mon(25, Some("SPARKY"))]);
        let out = party_summary(&save, &StubData);
        assert_eq!(out[0].nickname, Some("SPARKY".to_string()));
    }

    #[test]
    fn party_summary_no_nickname_is_none() {
        let save = save_with_party(vec![mon(25, None)]);
        let out = party_summary(&save, &StubData);
        assert_eq!(out[0].nickname, None);
    }

    #[test]
    fn party_summary_materializes_fainted_when_hp_zero() {
        let mut m = mon(25, None);
        m.hp = 0;
        let save = save_with_party(vec![m]);
        let out = party_summary(&save, &StubData);
        assert!(out[0].fainted);
        assert_eq!(out[0].status, Vec::<Condition>::new());
    }

    #[test]
    fn party_summary_not_fainted_when_hp_nonzero() {
        let save = save_with_party(vec![mon(25, None)]);
        let out = party_summary(&save, &StubData);
        assert!(!out[0].fainted);
    }

    #[test]
    fn party_summary_renders_paralyze_status() {
        let mut m = mon(25, None);
        m.status.paralyze = true;
        let save = save_with_party(vec![m]);
        let out = party_summary(&save, &StubData);
        assert_eq!(out[0].status, vec![Condition::Paralyze]);
    }

    #[test]
    fn party_summary_renders_sleep_with_turn_count() {
        let mut m = mon(25, None);
        m.status.sleep_turns = 3;
        let save = save_with_party(vec![m]);
        let out = party_summary(&save, &StubData);
        assert_eq!(out[0].status, vec![Condition::Sleep { turns: 3 }]);
    }

    #[test]
    fn party_summary_renders_multiple_conditions_in_order() {
        let mut m = mon(25, None);
        m.status.poison = true;
        m.status.burn = true;
        let save = save_with_party(vec![m]);
        let out = party_summary(&save, &StubData);
        assert_eq!(out[0].status, vec![Condition::Poison, Condition::Burn]);
    }

    #[test]
    fn party_summary_healthy_has_empty_status() {
        let save = save_with_party(vec![mon(25, None)]);
        let out = party_summary(&save, &StubData);
        assert_eq!(out[0].status, Vec::<Condition>::new());
    }

    #[test]
    fn resolve_returns_name_when_known() {
        assert_eq!(resolve(Some("MEWTWO"), 131), "MEWTWO");
    }

    #[test]
    fn resolve_falls_back_to_hash_id_when_unknown() {
        assert_eq!(resolve(None, 200), "#200");
    }

    #[test]
    fn party_summary_resolves_moves() {
        let mut m = mon(25, None);
        m.moves = vec![
            MoveSlot {
                move_id: 85,
                pp: 15,
                pp_ups: 2,
                slot: 0,
            },
            MoveSlot {
                move_id: 200,
                pp: 5,
                pp_ups: 0,
                slot: 2,
            },
        ];
        let save = save_with_party(vec![m]);
        let out = party_summary(&save, &StubData);
        assert_eq!(
            out[0].moves,
            vec![
                MoveView {
                    name: "THUNDERBOLT".to_string(),
                    pp: 15,
                    pp_ups: 2,
                    slot: 0
                },
                MoveView {
                    name: "#200".to_string(),
                    pp: 5,
                    pp_ups: 0,
                    slot: 2
                },
            ]
        );
    }

    #[test]
    fn party_summary_carries_stats_and_slots() {
        let save = save_with_party(vec![mon(131, None), mon(25, None)]);
        let out = party_summary(&save, &StubData);
        assert_eq!(out[1].slot, 1);
        assert_eq!(out[0].level, 50);
        assert_eq!(out[0].atk, 60);
        assert_eq!(out[0].def, 55);
        assert_eq!(out[0].spd, 70);
        assert_eq!(out[0].spc, 65);
        assert_eq!(out[0].hp, 100);
        assert_eq!(out[0].max_hp, 100);
    }

    #[test]
    fn items_view_resolves_names_and_quantity() {
        let items = vec![ItemStack {
            item_id: 1,
            quantity: 5,
        }];
        let out = items_view(&items, &StubData);
        assert_eq!(
            out,
            vec![ItemView {
                name: "MASTER BALL".to_string(),
                quantity: 5
            }]
        );
    }

    #[test]
    fn items_view_unknown_id_falls_back_to_hash_id() {
        let items = vec![ItemStack {
            item_id: 99,
            quantity: 3,
        }];
        let out = items_view(&items, &StubData);
        assert_eq!(
            out,
            vec![ItemView {
                name: "#99".to_string(),
                quantity: 3
            }]
        );
    }

    #[test]
    fn save_info_carries_fields() {
        let save = save_with_party(vec![mon(131, None)]);
        let info = save_info(&save);
        assert_eq!(
            info,
            SaveInfoView {
                trainer: "RED".to_string(),
                playtime: Playtime {
                    hours: 24,
                    minutes: 12
                },
                checksum_ok: true,
            }
        );
    }

    /// Integration: the REAL overlay plugs in and resolves id 131 -> MEWTWO.
    #[test]
    fn party_summary_with_real_yellow_legacy_overlay() {
        let save = save_with_party(vec![mon(131, None)]);
        let game = pokegen1::YellowLegacy::new();
        let out = party_summary(&save, &game);
        assert_eq!(out[0].species, "MEWTWO");
    }

    /// The selector wires the real overlay through the `dyn GameData` boundary:
    /// `game_data(YellowLegacy)` resolves the known id 131 -> MEWTWO.
    #[test]
    fn game_data_selects_yellow_legacy_overlay() {
        let game = game_data(GameId::YellowLegacy);
        assert_eq!(game.species_name(131), Some("MEWTWO"));
    }
}
