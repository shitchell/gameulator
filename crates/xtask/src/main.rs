//! `xtask` dev tool for the Gameulator project.
//!
//! Parses the pinned Yellow Legacy disassembly (`vendor/pokemon-yellow-legacy`,
//! tag V1.0.10) into generated JSON id->name tables consumed by the pokegen1
//! Yellow Legacy overlay. Run with:
//!
//! ```sh
//! cargo run -p xtask
//! ```
//!
//! Regenerate these tables after any ROM-version bump. The generated JSON is
//! committed so pokegen1 stays buildable without running xtask.

mod parsers;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Locate the workspace root by walking up from this crate's manifest dir until
/// the top-level workspace `Cargo.toml` (the one with `[workspace]`) is found.
fn workspace_root() -> Result<PathBuf> {
    // CARGO_MANIFEST_DIR points at crates/xtask at build time.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut dir = manifest.as_path();
    loop {
        let candidate = dir.join("Cargo.toml");
        if candidate.exists() {
            let text = std::fs::read_to_string(&candidate).unwrap_or_default();
            if text.contains("[workspace]") {
                return Ok(dir.to_path_buf());
            }
        }
        dir = dir
            .parent()
            .context("reached filesystem root without finding workspace Cargo.toml")?;
    }
}

/// Serialize an id->name map as a pretty-printed JSON object with stringified
/// keys. serde_json (without the `preserve_order` feature) stores object keys in
/// a `BTreeMap`, so keys are emitted in **lexical** string order ("1", "10",
/// "2", ...). That ordering is deterministic, which is all we need for stable
/// diffs across regenerations. Consumers key by id and do not rely on file
/// order.
fn to_json(map: &BTreeMap<u16, String>) -> Result<String> {
    let mut obj = serde_json::Map::new();
    for (id, name) in map {
        obj.insert(id.to_string(), serde_json::Value::String(name.clone()));
    }
    let value = serde_json::Value::Object(obj);
    let mut s = serde_json::to_string_pretty(&value)?;
    s.push('\n');
    Ok(s)
}

fn write_table(out_dir: &Path, name: &str, map: &BTreeMap<u16, String>) -> Result<()> {
    let path = out_dir.join(name);
    std::fs::write(&path, to_json(map)?)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn read(root: &Path, rel: &str) -> Result<String> {
    let path = root.join(rel);
    std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))
}

fn main() -> Result<()> {
    let root = workspace_root()?;
    let disasm = root.join("vendor/pokemon-yellow-legacy");
    anyhow::ensure!(
        disasm.is_dir(),
        "disassembly not found at {}",
        disasm.display()
    );

    let species = parsers::parse_species(&read(&disasm, "data/pokemon/names.asm")?);
    let moves = parsers::parse_moves(&read(&disasm, "data/moves/names.asm")?);
    let items = parsers::parse_items(
        &read(&disasm, "data/items/names.asm")?,
        &read(&disasm, "constants/item_constants.asm")?,
    );

    // Cross-check against read_save.py oracles (disassembly is ground truth).
    assert_eq!(
        species.get(&131).map(String::as_str),
        Some("MEWTWO"),
        "species internal id 131 must be MEWTWO"
    );
    assert_eq!(
        moves.get(&85).map(String::as_str),
        Some("THUNDERBOLT"),
        "move id 85 must be THUNDERBOLT"
    );
    assert_eq!(
        items.get(&1).map(String::as_str),
        Some("MASTER BALL"),
        "item id 1 must be MASTER BALL"
    );

    // Expected entry counts for the pinned ROM (tag V1.0.10). A spot-check on a
    // few ids catches wholesale re-indexing but NOT a truncated/over-long table,
    // so assert the counts too: this turns a silent disassembly-format drift on a
    // version bump into a loud failure at regeneration time (when a human is
    // present to reconcile it). Update these deliberately when bumping the ROM.
    anyhow::ensure!(
        species.len() == 190,
        "expected 190 species entries (V1.0.10), got {} — did the disassembly name-table format change?",
        species.len()
    );
    anyhow::ensure!(
        moves.len() == 165,
        "expected 165 move entries (V1.0.10), got {} — did the disassembly name-table format change?",
        moves.len()
    );
    anyhow::ensure!(
        items.len() == 138,
        "expected 138 item entries (83 regular + 5 HM + 50 TM, V1.0.10), got {} — did the disassembly item format change?",
        items.len()
    );

    let out_dir = root.join("crates/pokegen1/src/games/yellow_legacy/generated");
    std::fs::create_dir_all(&out_dir)
        .with_context(|| format!("creating {}", out_dir.display()))?;

    write_table(&out_dir, "species.json", &species)?;
    write_table(&out_dir, "moves.json", &moves)?;
    write_table(&out_dir, "items.json", &items)?;

    println!(
        "generated name tables from {}:\n  species.json: {} entries\n  moves.json:   {} entries\n  items.json:   {} entries",
        disasm.display(),
        species.len(),
        moves.len(),
        items.len()
    );

    Ok(())
}
