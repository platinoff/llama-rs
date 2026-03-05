//! Embeddings API: encode text to a vector (feature `embeddings`).
//!
//! Enable in Cargo.toml: `llama_rs = { version = "0.1", features = ["embeddings"] }`.

use super::{Context, Model};
use crate::error::{Error, Result};

/// Encode a string and return the last position's logits as an embedding vector.
///
/// Uses the model's forward pass (encode) on the tokenized input; returns the
/// last token's representation. The length of the returned vector is the
/// model's logits size (typically vocabulary size). For dedicated embedding
/// models, the effective dimension may be smaller; see the model card.
///
/// Requires feature `embeddings`.
#[cfg(feature = "embeddings")]
pub fn embed(model: &Model, context: &mut Context<'_>, text: &str) -> Result<Vec<f32>> {
    use llama_cpp_2::llama_batch::LlamaBatch;
    use llama_cpp_2::model::AddBos;

    let tokens = model
        .inner
        .str_to_token(text, AddBos::Always)
        .map_err(|e| Error::Tokenize(e.to_string()))?;

    if tokens.is_empty() {
        return Ok(Vec::new());
    }

    let n_batch = context.n_batch() as usize;
    let seq_id: i32 = 0;
    let mut batch = LlamaBatch::new(n_batch, 1);
    batch.clear();
    batch
        .add_sequence(&tokens, seq_id, false)
        .map_err(|e| Error::Decode(e.to_string()))?;
    context.encode(&mut batch)?;
    Ok(context.get_logits().to_vec())
}
