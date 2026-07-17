//! Save-level info header: trainer, playtime, and a checksum badge.

use leptos::*;

/// Renders the `<header class="info">` with trainer name, playtime, and a
/// checksum badge that toggles `ok`/`bad` classes for token coloring.
#[component]
pub fn InfoHeader(trainer: String, playtime: app::Playtime, checksum_ok: bool) -> impl IntoView {
    view! {
        <header class="info">
            <span class="trainer">{trainer}</span>
            <span class="playtime">{playtime.hours} "h " {playtime.minutes} "m"</span>
            <span class="badge" class:ok=checksum_ok class:bad=!checksum_ok>
                {if checksum_ok { "OK" } else { "BAD" }}
            </span>
        </header>
    }
}
