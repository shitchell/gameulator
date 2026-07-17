use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = web_server::WebConfig::parse();
    let addr = format!("0.0.0.0:{}", cfg.port);
    println!("[web] serving dashboard at http://localhost:{}", cfg.port);
    println!("[web]   status.json: {}", cfg.status_path.display());
    println!("[web]   dist:        {}", cfg.dist_dir.display());
    let app = web_server::router(cfg);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
