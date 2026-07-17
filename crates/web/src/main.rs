use std::time::Duration;

use gloo_net::http::Request;
use leptos::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <Dashboard/> });
}

/// The dashboard's fetch state for `/api/status`.
#[derive(Clone)]
enum Status {
    Loading,
    /// The watcher hasn't written a save yet (`{"present":false}`).
    Empty,
    /// A network error OR an unexpected/non-JSON response.
    Error(String),
    Loaded(app::StatusView),
}

/// Fetch `/api/status` once and classify the outcome into a [`Status`].
///
/// Parsing the body as an `app::StatusView` is the PRIMARY success signal — a
/// successful parse ALWAYS means `Loaded`, regardless of HTTP status. Only if
/// that fails do we distinguish the empty-state marker from an unexpected body,
/// so a non-JSON / HTML fallback surfaces as an `Error` instead of hanging.
async fn fetch_status() -> Status {
    let resp = match Request::get("/api/status").send().await {
        Ok(resp) => resp,
        Err(e) => return Status::Error(e.to_string()),
    };
    let text = match resp.text().await {
        Ok(text) => text,
        Err(e) => return Status::Error(e.to_string()),
    };

    if let Ok(sv) = serde_json::from_str::<app::StatusView>(&text) {
        return Status::Loaded(sv);
    }

    // Not a StatusView. Is it the empty-state marker (`{"present":false}`)?
    // Whitespace-tolerant: just require both tokens to be present.
    if text.contains("\"present\"") && text.contains("false") {
        return Status::Empty;
    }

    Status::Error("unexpected response from /api/status".to_string())
}

/// Fetch `poll_ms` from `/api/config` once at startup. Falls back to 2000 if the
/// request fails or the body doesn't parse — the frontend never blocks on config.
async fn fetch_poll_ms() -> u64 {
    const DEFAULT_POLL_MS: u64 = 2000;
    let Ok(resp) = Request::get("/api/config").send().await else {
        return DEFAULT_POLL_MS;
    };
    let Ok(text) = resp.text().await else {
        return DEFAULT_POLL_MS;
    };
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| v.get("poll_ms").and_then(serde_json::Value::as_u64))
        .unwrap_or(DEFAULT_POLL_MS)
}

/// Root component — fetches `/api/status`, renders loading/empty/error/loaded,
/// and re-fetches on the `poll_ms` interval so the page updates live as `sync`
/// rewrites `status.json`.
#[component]
fn Dashboard() -> impl IntoView {
    let (status, set_status) = create_signal(Status::Loading);

    // Immediate first fetch (don't wait a full interval).
    spawn_local(async move {
        set_status.set(fetch_status().await);
    });

    // Read poll_ms once, then start a repeating timer that re-fetches status.
    spawn_local(async move {
        let poll_ms = fetch_poll_ms().await;
        set_interval(
            move || {
                spawn_local(async move {
                    set_status.set(fetch_status().await);
                });
            },
            Duration::from_millis(poll_ms),
        );
    });

    move || match status.get() {
        Status::Loading => view! { <p class="state">"Loading…"</p> }.into_view(),
        Status::Empty => view! {
            <p class="state">"Waiting for a save… (start gameulator-sync and save in-game)"</p>
        }
        .into_view(),
        Status::Error(e) => view! { <p class="state error">"Error: " {e}</p> }.into_view(),
        Status::Loaded(sv) => view! {
            <h1>"Gameulator"</h1>
            <p>
                "Trainer: " {sv.trainer} " — " {sv.party.len()} " Pokémon — "
                {sv.playtime.hours} "h " {sv.playtime.minutes} "m"
            </p>
            <ul>
                {sv.party
                    .iter()
                    .map(|m| view! { <li>{m.species.clone()} " Lv" {m.level}</li> })
                    .collect_view()}
            </ul>
        }
        .into_view(),
    }
}
