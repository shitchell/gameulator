//! HP bar: a fill whose WIDTH is the hp ratio (inline % — data, not theming) and
//! whose COLOR comes from a ratio class (`hp-high`/`hp-mid`/`hp-low`) + tokens.

use leptos::*;

/// Renders the `<div class="hp-bar">` with an inner `hp-fill` sized to the hp
/// ratio and a `{hp}/{max_hp}` label. `max_hp == 0` is guarded (0% fill, `low`).
#[component]
pub fn HpBar(hp: u16, max_hp: u16) -> impl IntoView {
    let pct = if max_hp == 0 {
        0.0
    } else {
        (f64::from(hp) / f64::from(max_hp)) * 100.0
    };
    // Ratio class drives the color via tokens; width is the only inline style.
    let ratio_class = if pct > 50.0 {
        "hp-fill hp-high"
    } else if pct > 20.0 {
        "hp-fill hp-mid"
    } else {
        "hp-fill hp-low"
    };
    let style = format!("width: {pct}%");

    view! {
        <div class="hp-bar">
            <div class=ratio_class style=style></div>
        </div>
        <span class="hp-label">{hp} "/" {max_hp}</span>
    }
}
