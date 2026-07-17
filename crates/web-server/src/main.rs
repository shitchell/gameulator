use clap::Parser;
use web_server::WebConfig;

// Kept sync for now: this is a stub. Task 4 wires the axum server and will add
// `#[tokio::main]` when there's actually async work to run.
fn main() -> anyhow::Result<()> {
    let cfg = WebConfig::parse();
    println!("{cfg:?}");
    Ok(())
}
