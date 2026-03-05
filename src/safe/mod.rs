//! Safe wrappers around llama.cpp (100% Rust API).
//!
//! All types in this module are safe Rust; FFI is encapsulated in the `llama-cpp-2` dependency.

mod backend;
mod context;
mod generate;
mod model;

pub use backend::Backend;
pub use context::{Context, GenerateOptions, GenerateOptionsBuilder};
pub use generate::{generate, generate_stream};
#[cfg(feature = "metrics")]
pub use generate::generate_with_metrics;
pub use model::Model;
