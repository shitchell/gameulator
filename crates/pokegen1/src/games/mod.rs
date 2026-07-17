//! Game-specific overlays that implement the [`core::data`](crate::core::data)
//! traits.
//!
//! Each submodule owns the concrete name tables and constants for one
//! ROM/version; `core` stays table-free and depends only on the trait surface.

pub mod yellow_legacy;
