//! Status condition badges. Owns the Condition -> label + per-kind class mapping
//! (the controller keeps conditions structured; the view renders them).

use leptos::*;

/// Renders one `<span class="status-badge st-{kind}">` per condition. An empty
/// vec renders nothing.
#[component]
pub fn StatusBadges(status: Vec<app::Condition>) -> impl IntoView {
    status
        .into_iter()
        .map(|c| {
            let (label, kind) = match c {
                app::Condition::Sleep { turns } => (format!("SLEEP({turns})"), "sleep"),
                app::Condition::Poison => ("POISON".to_string(), "poison"),
                app::Condition::Burn => ("BURN".to_string(), "burn"),
                app::Condition::Freeze => ("FREEZE".to_string(), "freeze"),
                app::Condition::Paralyze => ("PARALYZE".to_string(), "paralyze"),
            };
            let class = format!("status-badge st-{kind}");
            view! { <span class=class>{label}</span> }
        })
        .collect_view()
}
