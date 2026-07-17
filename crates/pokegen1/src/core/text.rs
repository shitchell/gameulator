//! Gen-1 in-game string decoding.
//!
//! Pokémon Gen-1 stores text in a custom character encoding terminated by
//! byte `0x50` (or `0x00`). This module ports the `gbstr` decoder from
//! `reference/read_save.py` to match its known-good behavior.
//!
//! The authoritative long-term source for the charmap is the disassembly's
//! `charmap.asm`, but for now we match `read_save.py`.

/// Decode a Gen-1 in-game string.
///
/// Decoding stops at the first terminator byte (`0x50` or `0x00`). Bytes not
/// present in the charmap are skipped, and the result is trimmed of leading and
/// trailing whitespace.
pub fn decode_string(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &c in bytes {
        // Charmap — one contiguous block (ported from read_save.py `CH`/`gbstr`).
        // One conceptual change (the encoding) = one edit point.
        let ch = match c {
            0x50 | 0x00 => break, // terminators
            0x7F => " ",
            0x80..=0x99 => {
                // 0x80 = 'A' … 0x99 = 'Z'
                out.push((b'A' + (c - 0x80)) as char);
                continue;
            }
            0xE8 => ".",
            0x9A => "(",
            0x9B => ")",
            0x9C => ":",
            0xE3 => "-",
            0xF3 => "/",
            0xF4 => ",",
            0x9D => ";",
            _ => "", // not in map → skipped (matches Python CH.get(c, ""))
        };
        out.push_str(ch);
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_a_name() {
        // 0x80=A … 0x92=S, 0x87=H, 0x94=U, 0x8D=N; 0x50 stops, trailing 0xFF ignored.
        assert_eq!(decode_string(&[0x92, 0x87, 0x80, 0x94, 0x8D, 0x50, 0xFF]), "RED");
    }

    #[test]
    fn terminator_first_is_empty() {
        assert_eq!(decode_string(&[0x50]), "");
    }

    #[test]
    fn interior_space_is_preserved() {
        // "MR MIME": M=0x8C, R=0x91, space=0x7F, M=0x8C, I=0x88, M=0x8C, E=0x84
        assert_eq!(
            decode_string(&[0x8C, 0x91, 0x7F, 0x8C, 0x88, 0x8C, 0x84, 0x50]),
            "MR MIME"
        );
    }

    #[test]
    fn decodes_punctuation() {
        // "MR. MIME": M=0x8C, R=0x91, .=0xE8, space=0x7F, M=0x8C, I=0x88, M=0x8C, E=0x84
        assert_eq!(
            decode_string(&[0x8C, 0x91, 0xE8, 0x7F, 0x8C, 0x88, 0x8C, 0x84, 0x50]),
            "MR. MIME"
        );
    }

    #[test]
    fn decodes_full_alphabet() {
        let bytes: Vec<u8> = (0x80u8..=0x99).collect();
        assert_eq!(decode_string(&bytes), "ABCDEFGHIJKLMNOPQRSTUVWXYZ");
    }
}
