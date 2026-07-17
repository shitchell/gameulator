//! End-to-end wire-up test: build a synthetic save, run the `gameulator`
//! binary, and confirm the DTOs flow through to output. A bad checksum is fine
//! — `parse_save` returns `Ok` regardless (`checksum_ok` is just a flag), so we
//! never need to compute a valid one.

use std::io::Write;

use assert_cmd::Command;
use pokegen1::core::sram;
use predicates::prelude::*;
use tempfile::NamedTempFile;

/// Copy `bytes` into `buf` starting at `offset`.
fn seed(buf: &mut [u8], offset: usize, bytes: &[u8]) {
    buf[offset..offset + bytes.len()].copy_from_slice(bytes);
}

/// Build a minimal 32 KiB SRAM buffer: one party mon (MEWTWO, id 131) plus a
/// trainer name. Uses only the PUBLIC `sram` offset consts.
fn synthetic_save() -> Vec<u8> {
    let mut buf = vec![0u8; sram::SAVE_LEN];

    // Trainer "RED" (Gen-1 charmap, 0x50-terminated).
    seed(&mut buf, sram::NAME, &[0x91, 0x84, 0x83, 0x50]);

    // One Pokémon in the party.
    seed(&mut buf, sram::PARTY_COUNT, &[1]);
    seed(&mut buf, sram::PARTY_SPECIES, &[131]);

    // The 44-byte party struct at PARTY_DATA. Field offsets are relative to the
    // struct base; the parser reads species from +0x00 and level from +0x21.
    let base = sram::PARTY_DATA;
    seed(&mut buf, base, &[131]); // +0x00 species = MEWTWO
    seed(&mut buf, base + 0x01, &[0x00, 0x64]); // cur HP = 100 (BE, non-zero => not fainted)
    seed(&mut buf, base + 0x21, &[70]); // +0x21 party level = 70
    seed(&mut buf, base + 0x22, &[0x00, 0xB4]); // max HP = 180 (BE)

    buf
}

fn write_save() -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(&synthetic_save()).unwrap();
    f.flush().unwrap();
    f
}

#[test]
fn party_json_resolves_species_name() {
    let save = write_save();
    Command::cargo_bin("gameulator")
        .unwrap()
        .args(["party", save.path().to_str().unwrap(), "--json"])
        .assert()
        .success()
        // id 131 resolves to MEWTWO via the real YellowLegacy overlay.
        .stdout(predicate::str::contains("MEWTWO"));
}

#[test]
fn info_runs_and_shows_trainer() {
    let save = write_save();
    Command::cargo_bin("gameulator")
        .unwrap()
        .args(["info", save.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("RED"));
}
