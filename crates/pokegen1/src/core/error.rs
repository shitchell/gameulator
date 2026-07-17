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
}
