//! Assistants (beta v2) types, mirroring `openai-python/src/openai/types/beta/`.
//!
//! Split into [`responses`] (typed response models, `RunStatus`, content
//! blocks, deleted markers, list query params) and [`params`] (request
//! bodies). Streaming runs are intentionally out of scope.

pub mod params;
pub mod responses;

pub use params::*;
pub use responses::*;
