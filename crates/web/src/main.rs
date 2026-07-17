use gloo_net::http::Request;
use leptos::*;

mod components;
use components::{InfoHeader, ItemList, PartyCard};

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

/// Root component — polls `/api/status` on a self-rescheduling loop and renders
/// the dashboard from the last successful load, so a transient error never
/// blanks a working page.
///
/// Two signals cooperate:
/// - `status` is the CURRENT fetch outcome (used for the loading/empty/error
///   states and the non-destructive error banner).
/// - `last_good` holds the most recent `Loaded` [`app::StatusView`] so the full
///   dashboard keeps rendering across transient errors.
///
/// The poll is a `spawn_local` loop that awaits each fetch before scheduling the
/// next `TimeoutFuture`, so fetches can never overlap and no timer is leaked.
#[component]
fn Dashboard() -> impl IntoView {
    let (status, set_status) = create_signal(Status::Loading);
    let (last_good, set_last_good) = create_signal::<Option<app::StatusView>>(None);

    // This loop is never cancelled — safe because `Dashboard` is the root
    // component (mounted via `mount_to_body`) and never unmounts. If it ever
    // becomes an unmountable child, cancel this on `on_cleanup`.
    spawn_local(async move {
        let poll_ms = fetch_poll_ms().await;
        loop {
            let s = fetch_status().await;
            // Keep-last-good: a successful load updates BOTH signals so the
            // dashboard survives later transient errors.
            if let Status::Loaded(sv) = &s {
                set_last_good.set(Some(sv.clone()));
            }
            set_status.set(s);
            gloo_timers::future::TimeoutFuture::new(poll_ms as u32).await;
        }
    });

    move || match last_good.get() {
        // We have good data: render the full dashboard, and overlay a small
        // non-destructive banner if the CURRENT fetch is an error.
        Some(sv) => {
            let banner = match status.get() {
                Status::Error(e) => view! { <div class="error-banner">"⚠ " {e}</div> }.into_view(),
                _ => ().into_view(),
            };
            view! {
                <h1>"Gameulator"</h1>
                {banner}
                <InfoHeader
                    trainer=sv.trainer.clone()
                    playtime=sv.playtime.clone()
                    checksum_ok=sv.checksum_ok
                />
                <div class="party-grid">
                    {sv.party
                        .iter()
                        .map(|m| view! { <PartyCard mon=m.clone()/> })
                        .collect_view()}
                </div>
                <ItemList title="Bag".to_string() items=sv.bag.clone()/>
                <ItemList title="PC".to_string() items=sv.pc.clone()/>
            }
            .into_view()
        }
        // No good data yet: show the plain state line for the current outcome.
        None => match status.get() {
            Status::Loading => view! { <p class="state">"Loading…"</p> }.into_view(),
            Status::Empty => view! {
                <p class="state">"Waiting for a save… (start gameulator-sync and save in-game)"</p>
            }
            .into_view(),
            Status::Error(e) => view! { <p class="state error">"Error: " {e}</p> }.into_view(),
            // Loaded but last_good not yet set (same tick) — treat as loading.
            Status::Loaded(_) => view! { <p class="state">"Loading…"</p> }.into_view(),
        },
    }
}
