//! A single party member card, composed of the smaller components.

use leptos::*;

use crate::components::{HpBar, MoveList, StatusBadges};

/// Renders an `<article class="party-card">` (with `fainted` toggled) for one
/// party member: title, level, HP bar, stats row, status badges, and moves.
///
/// Note: a fainted mon shows BOTH the `fainted` card styling AND any status
/// badges — a deliberate divergence from the CLI (which collapses a fainted mon
/// to just "FAINTED", discarding status bits). Here the class conveys fainted and
/// the badges convey conditions; keep them independent.
#[component]
pub fn PartyCard(mon: app::PartyMemberView) -> impl IntoView {
    let title = match &mon.nickname {
        Some(nick) => format!("{nick} ({})", mon.species),
        None => mon.species.clone(),
    };

    view! {
        <article class="party-card" class:fainted=mon.fainted>
            <h3 class="title">{title}</h3>
            <span class="level">"Lv" {mon.level}</span>
            <HpBar hp=mon.hp max_hp=mon.max_hp/>
            <div class="stats">
                <span>"Atk " {mon.atk}</span>
                <span>"Def " {mon.def}</span>
                <span>"Spd " {mon.spd}</span>
                <span>"Spc " {mon.spc}</span>
            </div>
            <StatusBadges status=mon.status.clone()/>
            <MoveList moves=mon.moves.clone()/>
        </article>
    }
}
