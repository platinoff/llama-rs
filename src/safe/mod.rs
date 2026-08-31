//! Safe wrappers around llama.cpp (100% Rust API).
//!
//! All types in this module are safe Rust; FFI is encapsulated in the `llama-cpp-2` dependency.

mod backend;
mod context;
mod embed;
mod generate;
mod model;
mod staged;

pub use backend::Backend;
pub use context::{presets as context_presets, Context, GenerateOptions, GenerateOptionsBuilder};
#[cfg(feature = "embeddings")]
pub use embed::{embed, embed_normalized};
pub use embed::{l2_norm, l2_normalize, mean_pool};
#[cfg(feature = "metrics")]
pub use generate::generate_with_metrics;
pub use generate::{generate, generate_stream};
pub use model::Model;
pub use staged::StagedLoadOptions;
