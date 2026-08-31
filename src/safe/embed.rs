//! Embeddings API: encode text to a vector (feature `embeddings`) + L2/mean-pool helpers.
//!
//! Enable in Cargo.toml: `llama_rs = { version = "0.1", features = ["embeddings"] }`.
//! Helpers `l2_normalize` / `l2_norm` are pure Rust and always available (no feature).

#[cfg(feature = "embeddings")]
use super::{Context, Model};
#[cfg(feature = "embeddings")]
use crate::error::{Error, Result};

/// L2 norm of a vector (pure Rust, no deps).
#[must_use]
pub fn l2_norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

/// In-place L2 normalization. No-op for empty or zero-norm vectors.
pub fn l2_normalize(v: &mut [f32]) {
    let n = l2_norm(v);
    if n > 0.0 && n.is_finite() {
        for x in v.iter_mut() {
            *x /= n;
        }
    }
}

/// Mean-pool a slice of vectors (e.g. per-token embeddings) into one vector.
/// `vecs` is a slice of equal-length vectors; returns their element-wise mean.
/// Empty input → empty vec.
#[must_use]
pub fn mean_pool(vecs: &[Vec<f32>]) -> Vec<f32> {
    if vecs.is_empty() {
        return Vec::new();
    }
    let dim = vecs[0].len();
    let mut out = vec![0.0f32; dim];
    for v in vecs {
        debug_assert_eq!(v.len(), dim, "mean_pool: all vectors must have same dim");
        for (o, x) in out.iter_mut().zip(v.iter()) {
            *o += *x;
        }
    }
    let n = vecs.len() as f32;
    for o in out.iter_mut() {
        *o /= n;
    }
    out
}

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

/// Like [`embed`] but returns an L2-normalized vector (unit length).
/// Requires feature `embeddings`.
#[cfg(feature = "embeddings")]
pub fn embed_normalized(model: &Model, context: &mut Context<'_>, text: &str) -> Result<Vec<f32>> {
    let mut v = embed(model, context, text)?;
    l2_normalize(&mut v);
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l2_norm_basic() {
        let v = [3.0, 4.0];
        assert!((l2_norm(&v) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn l2_normalize_unit() {
        let mut v = vec![3.0, 4.0];
        l2_normalize(&mut v);
        assert!((l2_norm(&v) - 1.0).abs() < 1e-6);
        assert!((v[0] - 0.6).abs() < 1e-6);
    }

    #[test]
    fn l2_normalize_empty_noop() {
        let mut v: Vec<f32> = vec![];
        l2_normalize(&mut v);
        assert!(v.is_empty());
    }

    #[test]
    fn mean_pool_basic() {
        let a = vec![1.0, 2.0];
        let b = vec![3.0, 4.0];
        let m = mean_pool(&[a, b]);
        assert_eq!(m, vec![2.0, 3.0]);
    }

    #[test]
    fn mean_pool_empty() {
        let m = mean_pool(&[]);
        assert!(m.is_empty());
    }
}
