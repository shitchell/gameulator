use leptos::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <Dashboard/> });
}

/// Root component — a placeholder for now; the fetch/poll + party render land in
/// Tasks 6-8.
#[component]
fn Dashboard() -> impl IntoView {
    view! { <h1>"Gameulator"</h1> }
}
