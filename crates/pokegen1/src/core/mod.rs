//! Core Gen-1 model logic (the Model layer).

pub mod checksum;
pub mod error;
pub mod header;
pub mod items;
pub mod party;
pub mod pokemon;
pub mod sram;
pub mod text;

#[cfg(test)]
pub(crate) mod test_support;
