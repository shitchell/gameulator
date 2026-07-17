//! Variant-data lookup traits — the **core↔overlay seam**.
//!
//! `pokegen1::core` is game-agnostic: it parses a save into numeric ids
//! (species id, move id, item id) and never carries a lookup table. The *names*
//! for those ids are game-specific (a Yellow Legacy ROM lays them out one way, a
//! vanilla ROM another), so they live in an **overlay** crate that IMPLEMENTS the
//! traits defined here.
//!
//! # Locality rationale
//! One place (this module) defines the interface; overlays fill it. Keeping the
//! trait definitions in `core` — rather than in each overlay — means every
//! consumer (the app controller, a CLI renderer) depends on a single, stable
//! surface, and the ROM-version data-generation step (Task 13) produces the
//! concrete tables that satisfy it. The core stays table-free; the overlay stays
//! the sole owner of game data.
//!
//! # Scope note
//! Only NAME resolution lives here — that is all Milestone-1's CLI needs.
//! A `TypeChart` trait / `Type` enum (for the future type-coverage feature) are
//! deliberately **out of scope** and are not defined here.

/// Resolve a Gen-1 species id (1-based dex-ish index as stored in the save) to a
/// name. Unknown ids return `None`. The returned borrow is tied to `&self`, so
/// both owned-`String` tables and `&'static` stubs satisfy it.
pub trait SpeciesTable {
    fn species_name(&self, id: u8) -> Option<&str>;
}

/// Resolve a move id to a name. Unknown ids return `None`.
pub trait MoveTable {
    fn move_name(&self, id: u8) -> Option<&str>;
}

/// Resolve an item id to a name. Unknown ids return `None`.
pub trait ItemTable {
    fn item_name(&self, id: u8) -> Option<&str>;
}

/// Aggregate: a game data source that can resolve species, moves, and items.
///
/// Consumers that need all three (e.g. the app controller) take
/// `&impl GameData` (or `&dyn GameData`); each individual lookup can also be
/// required on its own via the constituent traits.
pub trait GameData: SpeciesTable + MoveTable + ItemTable {}

/// Blanket impl so any type implementing all three tables is automatically
/// [`GameData`] — overlays never implement `GameData` explicitly.
impl<T: SpeciesTable + MoveTable + ItemTable> GameData for T {}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny hardcoded stub standing in for a real overlay, resolving exactly
    /// one id per table so the tests can prove both the `Some(..)` and `None`
    /// paths without any generated data.
    struct StubData;

    impl SpeciesTable for StubData {
        fn species_name(&self, id: u8) -> Option<&str> {
            match id {
                131 => Some("MEWTWO"),
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

    #[test]
    fn species_name_resolves_known_and_unknown() {
        let data = StubData;
        assert_eq!(data.species_name(131), Some("MEWTWO"));
        assert_eq!(data.species_name(0), None);
        assert_eq!(data.species_name(254), None);
    }

    #[test]
    fn move_name_resolves_known_and_unknown() {
        let data = StubData;
        assert_eq!(data.move_name(85), Some("THUNDERBOLT"));
        assert_eq!(data.move_name(0), None);
        assert_eq!(data.move_name(254), None);
    }

    #[test]
    fn item_name_resolves_known_and_unknown() {
        let data = StubData;
        assert_eq!(data.item_name(1), Some("MASTER BALL"));
        assert_eq!(data.item_name(0), None);
        assert_eq!(data.item_name(254), None);
    }

    /// Prove the supertrait + blanket impl compose: a fn taking `&dyn GameData`
    /// can reach all three lookups through the single aggregate bound, and the
    /// stub (which only implements the three constituent traits) is accepted.
    fn all_three(g: &dyn GameData) -> (Option<&str>, Option<&str>, Option<&str>) {
        (g.species_name(131), g.move_name(85), g.item_name(1))
    }

    #[test]
    fn stub_is_usable_through_game_data_aggregate() {
        let data = StubData;
        assert_eq!(
            all_three(&data),
            (Some("MEWTWO"), Some("THUNDERBOLT"), Some("MASTER BALL"))
        );
    }
}
