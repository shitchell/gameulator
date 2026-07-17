//! Crate error type for Gen-1 save parsing.
//!
//! Kept intentionally minimal (YAGNI): variants are added as parsers that
//! need them land. Every fallible `core` parser returns [`ParseError`].

/// Errors that can occur while parsing a Gen-1 save.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ParseError {
    /// The party-count byte was outside the valid `1..=6` range — the primary
    /// "is this actually a Gen-1 party save?" sanity check.
    #[error("invalid party count: {0} (expected 1..=6)")]
    InvalidPartyCount(u8),

    /// The save buffer was shorter than a full Gen-1 SRAM dump. Detected by
    /// `parse_save`'s length gate BEFORE any offset accessor runs, so a
    /// short/corrupt buffer never panics on a direct index.
    #[error("save too short: expected at least {expected} bytes, got {got}")]
    TruncatedSave { expected: usize, got: usize },
}
