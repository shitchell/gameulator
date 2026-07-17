use anyhow::Context;
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = web_server::WebConfig::parse();
    // No auth on this dashboard — bind localhost so it's not exposed to the LAN.
    let port = cfg.port;
    let addr = format!("127.0.0.1:{port}");
    println!("[web] serving dashboard at http://localhost:{port}");
    println!("[web]   status.json: {}", cfg.status_path.display());
    println!("[web]   dist:        {}", cfg.dist_dir.display());
    let app = web_server::router(cfg);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| {
            format!(
                "failed to bind {addr} — is another gameulator-web running, or is port {port} in use?"
            )
        })?;
    axum::serve(listener, app).await?;
    Ok(())
}
