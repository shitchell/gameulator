//! axum backend for the Gameulator web dashboard: serves the built WASM
//! frontend + a small JSON API (/api/status, /api/config) over the sync
//! watcher's status.json.

use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use clap::Parser;

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

/// Build the axum router for a given config. (Static-asset serving is added in
/// the next task; this task adds the JSON API.)
pub fn router(cfg: WebConfig) -> Router {
    let state = Arc::new(cfg);
    Router::new()
        .route("/api/status", get(status_handler))
        .with_state(state)
}

/// GET /api/status — returns the sync watcher's status.json if present, else an
/// empty-state marker. Always 200 (an absent file is a normal "watcher hasn't
/// written yet" state, not an error — the frontend shows a waiting message).
async fn status_handler(
    axum::extract::State(cfg): axum::extract::State<Arc<WebConfig>>,
) -> Response {
    match std::fs::read(&cfg.status_path) {
        Ok(bytes) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            bytes,
        )
            .into_response(),
        Err(_) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"present":false}"#,
        )
            .into_response(),
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
}
