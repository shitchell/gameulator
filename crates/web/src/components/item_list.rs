//! An item list section (bag or PC).

use leptos::*;

/// Renders a `<section class="item-list">` with a titled count and a `<ul>` of
/// `{name} x{quantity}` items; an empty list renders a muted `(none)`.
#[component]
pub fn ItemList(title: String, items: Vec<app::ItemView>) -> impl IntoView {
    let len = items.len();
    let body = if items.is_empty() {
        view! { <p class="muted">"(none)"</p> }.into_view()
    } else {
        view! {
            <ul>
                {items
                    .into_iter()
                    .map(|it| view! { <li>{it.name} " x" {it.quantity}</li> })
                    .collect_view()}
            </ul>
        }
        .into_view()
    };

    view! {
        <section class="item-list">
            <h2>{title} " (" {len} ")"</h2>
            {body}
        </section>
    }
}
