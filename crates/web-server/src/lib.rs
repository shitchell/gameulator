//! axum backend for the Gameulator web dashboard: serves the built WASM
//! frontend + a small JSON API (/api/status, /api/config) over the sync
//! watcher's status.json.

use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use clap::Parser;
use serde::Serialize;
use tower_http::services::{ServeDir, ServeFile};

/// Runtime config for the web dashboard server.
#[derive(Debug, Clone, Parser)]
#[command(name = "gameulator-web")]
pub struct WebConfig {
    /// Path to the sync watcher's status.json.
    #[arg(long, default_value = "games/Pokemon/Yellow Legacy/saves/status.json")]
    pub status_path: PathBuf,
    /// Directory of the built frontend assets (trunk dist).
    #[arg(long, default_value = "crates/web/dist")]
    pub dist_dir: PathBuf,
    /// Port to bind.
    #[arg(long, default_value_t = 8770)]
    pub port: u16,
    /// Poll interval (ms) the frontend uses to re-fetch /api/status.
    #[arg(long, default_value_t = 2000)]
    pub poll_ms: u32,
}

/// Build the axum router for a given config. Explicit `/api/*` routes take
/// precedence; everything else falls through to the SPA static assets.
pub fn router(cfg: WebConfig) -> Router {
    // SPA static serving: serve files from `dist_dir`, falling back to
    // `dist_dir/index.html` so `/` loads the app and client-side routes resolve.
    // ServeDir/ServeFile are lazy — a nonexistent `dist_dir` does NOT panic at
    // construction; it just yields 404s until `trunk build` creates the dir.
    let index = cfg.dist_dir.join("index.html");
    let static_service = ServeDir::new(&cfg.dist_dir).fallback(ServeFile::new(index));

    let state = Arc::new(cfg);
    Router::new()
        .route("/api/status", get(status_handler))
        .route("/api/config", get(config_handler))
        .with_state(state)
        .fallback_service(static_service)
}

/// JSON payload for GET /api/config.
#[derive(Serialize)]
struct ConfigResponse {
    poll_ms: u32,
}

/// GET /api/config — exposes the frontend's poll interval so it has ONE source
/// (WebConfig::poll_ms) rather than a hard-coded value in the JS.
async fn config_handler(
    axum::extract::State(cfg): axum::extract::State<Arc<WebConfig>>,
) -> Json<ConfigResponse> {
    Json(ConfigResponse {
        poll_ms: cfg.poll_ms,
    })
}

/// GET /api/status — returns the sync watcher's status.json if present, else an
/// empty-state marker. Always 200 (an absent file is a normal "watcher hasn't
/// written yet" state, not an error — the frontend shows a waiting message).
async fn status_handler(
    axum::extract::State(cfg): axum::extract::State<Arc<WebConfig>>,
) -> Response {
    // Sync read: status.json is tiny and this endpoint is low-QPS (one poll every
    // ~2s per tab), so blocking the worker here is fine — no spawn_blocking needed.
    match std::fs::read(&cfg.status_path) {
        Ok(bytes) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            bytes,
        )
            .into_response(),
        Err(e) => {
            // A missing file is the normal "watcher hasn't written yet" state —
            // stay quiet. Any OTHER error (permissions, EIO, a bad path) would
            // otherwise masquerade as "waiting" forever, so log it — while still
            // returning the same empty-state so a transient hiccup can't blank the
            // dashboard.
            if e.kind() != std::io::ErrorKind::NotFound {
                eprintln!("[web] reading {} failed: {e}", cfg.status_path.display());
            }
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/json")],
                r#"{"present":false}"#,
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use tower::ServiceExt; // for `.oneshot`

    #[test]
    fn web_config_defaults() {
        let cfg = WebConfig::parse_from(["gameulator-web"]);
        assert_eq!(
            cfg.status_path,
            PathBuf::from("games/Pokemon/Yellow Legacy/saves/status.json")
        );
        assert_eq!(cfg.dist_dir, PathBuf::from("crates/web/dist"));
        assert_eq!(cfg.port, 8770);
        assert_eq!(cfg.poll_ms, 2000);
    }

    async fn body_string(resp: axum::response::Response) -> String {
        let bytes = http_body_util::BodyExt::collect(resp.into_body())
            .await
            .unwrap()
            .to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    fn cfg_with_status_path(status_path: PathBuf) -> WebConfig {
        let mut cfg = WebConfig::parse_from(["gameulator-web"]);
        cfg.status_path = status_path;
        cfg
    }

    #[tokio::test]
    async fn status_present_serves_file_bytes() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), r#"{"trainer":"RED","checksum_ok":true}"#).unwrap();

        let app = router(cfg_with_status_path(tmp.path().to_path_buf()));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        let body = body_string(resp).await;
        assert!(body.contains("\"RED\""), "body was: {body}");
    }

    #[tokio::test]
    async fn status_absent_returns_empty_state_marker() {
        let missing = PathBuf::from("/nonexistent/definitely/not/here/status.json");
        assert!(!missing.exists());

        let app = router(cfg_with_status_path(missing));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        let body = body_string(resp).await;
        assert_eq!(body, r#"{"present":false}"#);
    }

    #[tokio::test]
    async fn api_config_returns_poll_ms() {
        // Default poll_ms is 2000; the frontend reads this as its single source
        // of the poll interval.
        let cfg = WebConfig::parse_from(["gameulator-web"]);
        assert_eq!(cfg.poll_ms, 2000);

        let app = router(cfg);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/config")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["poll_ms"], 2000);
    }

    fn cfg_with_dist_dir(dist_dir: PathBuf) -> WebConfig {
        let mut cfg = WebConfig::parse_from(["gameulator-web"]);
        cfg.dist_dir = dist_dir;
        cfg
    }

    #[tokio::test]
    async fn serves_index_html_at_root() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("index.html"),
            "<!doctype html><title>Gameulator</title>",
        )
        .unwrap();

        let app = router(cfg_with_dist_dir(tmp.path().to_path_buf()));
        let resp = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp).await;
        assert!(
            body.contains("<title>Gameulator</title>"),
            "body was: {body}"
        );
    }

    #[tokio::test]
    async fn serves_static_asset() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("index.html"), "<title>Gameulator</title>").unwrap();
        std::fs::write(tmp.path().join("style.css"), "body{color:red}").unwrap();

        let app = router(cfg_with_dist_dir(tmp.path().to_path_buf()));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/style.css")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp).await;
        assert_eq!(body, "body{color:red}");
    }

    #[tokio::test]
    async fn missing_dist_dir_does_not_panic_and_404s() {
        // Task-2 review carry-forward: ServeDir/ServeFile are lazy, so a
        // nonexistent dist_dir must NOT panic at router construction; it just
        // yields 404s until `trunk build` creates the dir.
        let missing = PathBuf::from("/nonexistent/definitely/not/here/dist");
        assert!(!missing.exists());

        // Must not panic:
        let app = router(cfg_with_dist_dir(missing));
        let resp = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert!(
            !resp.status().is_success(),
            "expected non-2xx, got {}",
            resp.status()
        );
    }
}
