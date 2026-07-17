//! A party member's move list.

use leptos::*;

/// Renders a `<ul class="moves">` of `{name} (pp)` list items.
#[component]
pub fn MoveList(moves: Vec<app::MoveView>) -> impl IntoView {
    view! {
        <ul class="moves">
            {moves
                .into_iter()
                .map(|m| view! { <li>{m.name} " (" {m.pp} ")"</li> })
                .collect_view()}
        </ul>
    }
}
