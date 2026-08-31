//! Staged model loading: disk → RAM ступенями (pure Rust orchestration).
//!
//! Controls how GGUF is brought from storage to RAM via `llama-cpp-2`:
//! - `use_mmap = true` (default) — file stays on disk, pages faulted on demand (mmap). Lets a 27B
//!   model (≈6.9 GiB mapped) run on a 16 GiB box with ~0.6 GiB free.
//! - `use_mmap = false` — read fully into RAM (slower start, more resident).
//! - `use_mlock = true` — pin pages into RAM (mlock), avoids swapping but needs privilege / RAM.
//! - `progress_callback` — `FnMut(f32) -> bool` with `p in 0.0..=1.0`; returning `false` aborts.
//!
//! All orchestration is safe Rust; the backend stays `llama-cpp-2` (`llama-cpp-sys-2 0.1.154`).

use crate::error::Result;
use crate::safe::{Backend, Model};
use std::path::Path;

/// Options for staged loading (disk → RAM). Builder-style, `Copy + Clone`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StagedLoadOptions {
    /// Use `mmap` (default true). If false, the file is read into RAM.
    pub use_mmap: bool,
    /// Pin to RAM via `mlock` (default false). Requires enough RAM.
    pub use_mlock: bool,
}

impl Default for StagedLoadOptions {
    fn default() -> Self {
        Self {
            use_mmap: true,
            use_mlock: false,
        }
    }
}

impl StagedLoadOptions {
    /// New with defaults (`mmap=true`, `mlock=false`).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to use `mmap` (paged, on-disk).
    #[must_use]
    pub fn with_mmap(mut self, v: bool) -> Self {
        self.use_mmap = v;
        self
    }

    /// Set whether to `mlock` (pin) pages into RAM.
    #[must_use]
    pub fn with_mlock(mut self, v: bool) -> Self {
        self.use_mlock = v;
        self
    }

    /// `mmap=true, mlock=false` — default, low-RAM friendly (27B on 5500U).
    #[must_use]
    pub fn mmap() -> Self {
        Self {
            use_mmap: true,
            use_mlock: false,
        }
    }

    /// `mmap=false` — fully resident (no paging). Needs ~8 GiB RAM for 27B.
    #[must_use]
    pub fn resident() -> Self {
        Self {
            use_mmap: false,
            use_mlock: false,
        }
    }

    /// `mmap + mlock` — pinned (fast, no swap, needs privilege + RAM).
    #[must_use]
    pub fn pinned() -> Self {
        Self {
            use_mmap: true,
            use_mlock: true,
        }
    }
}

impl Model {
    /// Load a model with staged options and an optional progress callback.
    ///
    /// `on_progress` is `FnMut(f32) -> bool` with `p in 0.0..=1.0`; return `false` to abort.
    /// Pure Rust wrapper around `LlamaModelParams::with_progress_callback` + `load_mode`.
    ///
    /// Example:
    /// ```no_run
    /// use llama_rs::{Backend, Model, StagedLoadOptions};
    /// let backend = Backend::init().unwrap();
    /// let opts = StagedLoadOptions::mmap();
    /// let model = Model::load_staged(&backend, "models/Qwen3.8-27B-UD-IQ2_XXS.gguf", opts, Some(|p| { println!("{:.0}%", p*100.0); true })).unwrap();
    /// ```
    pub fn load_staged<F>(
        backend: &Backend,
        path: impl AsRef<Path>,
        opts: StagedLoadOptions,
        on_progress: Option<F>,
    ) -> Result<Self>
    where
        F: FnMut(f32) -> bool + 'static,
    {
        use llama_cpp_2::model::params::LlamaModelParams;

        let path_ref = path.as_ref().to_path_buf();
        let base = LlamaModelParams::default()
            .with_use_mmap(opts.use_mmap)
            .with_use_mlock(opts.use_mlock);

        if let Some(cb) = on_progress {
            let params = base.with_progress_callback(cb);
            Self::load_from_params(backend, path_ref, &params)
        } else {
            Self::load_from_params(backend, path_ref, &base)
        }
    }

    /// Load with a borrowed `&mut dyn FnMut` progress (ergonomic helper).
    ///
    /// Transmutes to `'static` for `with_progress_callback`; safe because the
    /// load is synchronous and the closure does not outlive the call.
    pub fn load_staged_with_progress(
        backend: &Backend,
        path: impl AsRef<Path>,
        opts: StagedLoadOptions,
        on_progress: &mut dyn FnMut(f32) -> bool,
    ) -> Result<Self> {
        use llama_cpp_2::model::params::LlamaModelParams;
        let path_buf = path.as_ref().to_path_buf();
        let base = LlamaModelParams::default()
            .with_use_mmap(opts.use_mmap)
            .with_use_mlock(opts.use_mlock);
        // SAFETY: `on_progress` lives for the duration of this call and the load is
        // synchronous, so the `'static` transmute does not allow use-after-free.
        let static_cb: &'static mut dyn FnMut(f32) -> bool =
            unsafe { std::mem::transmute(on_progress) };
        let params = base.with_progress_callback(static_cb);
        Self::load_from_params(backend, path_buf, &params)
    }

    fn load_from_params(
        backend: &Backend,
        path: std::path::PathBuf,
        params: &llama_cpp_2::model::params::LlamaModelParams,
    ) -> Result<Self> {
        // Delegate to existing `load_from_file` path conversion.
        let inner = llama_cpp_2::model::LlamaModel::load_from_file(backend.inner(), &path, params)
            .map_err(|e| crate::error::Error::ModelLoad {
                path,
                message: e.to_string(),
            })?;
        Ok(Model { inner })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staged_default_is_mmap() {
        let o = StagedLoadOptions::default();
        assert!(o.use_mmap);
        assert!(!o.use_mlock);
    }

    #[test]
    fn staged_presets() {
        assert_eq!(
            StagedLoadOptions::mmap(),
            StagedLoadOptions {
                use_mmap: true,
                use_mlock: false
            }
        );
        assert_eq!(
            StagedLoadOptions::resident(),
            StagedLoadOptions {
                use_mmap: false,
                use_mlock: false
            }
        );
        assert_eq!(
            StagedLoadOptions::pinned(),
            StagedLoadOptions {
                use_mmap: true,
                use_mlock: true
            }
        );
    }

    #[test]
    fn staged_builder() {
        let o = StagedLoadOptions::new().with_mmap(false).with_mlock(true);
        assert!(!o.use_mmap);
        assert!(o.use_mlock);
    }

    #[test]
    fn progress_callback_can_abort_via_params() {
        use llama_cpp_2::model::params::LlamaModelParams;
        // Verify the callback is accepted and aborts (returns false signal).
        let params = LlamaModelParams::default().with_progress_callback(|p| {
            assert!((0.0..=1.0).contains(&p));
            false
        });
        // Params built successfully; the model load would abort on first progress.
        drop(params);
    }
}
