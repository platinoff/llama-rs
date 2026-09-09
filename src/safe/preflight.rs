//! Preflight advice before model load: free RAM vs model file size (pure Rust).
//!
//! Diagnoses the known thrash mode from `docs/PERFORMANCE_RESEARCH.md`: with
//! `mmap` and free RAM far below the GGUF file size, weight pages refault from
//! disk on every decode step (0.03 tok/s on a box that should do ~1 tok/s).
//! Free-RAM reads use `sysinfo` (safe, cross-platform); all decision logic is
//! pure and unit-tested with injected sizes.

use crate::safe::StagedLoadOptions;
use std::path::Path;

/// Rough headroom (MiB) a resident/pinned load needs over the model file for
/// KV cache + compute buffers + runtime (in addition to the mapped weights).
pub const RESIDENT_HEADROOM_MIB: u64 = 1024;

/// Below this many free MiB always warn, regardless of model size.
pub const CRITICAL_FREE_MIB: u64 = 512;

/// Model GGUF size in MiB from disk metadata. `None` when unreadable.
#[must_use]
pub fn model_file_mib(path: &Path) -> Option<u64> {
    let len = std::fs::metadata(path).ok()?.len();
    Some(len / (1024 * 1024))
}

/// Free RAM in MiB. `None` when the system cannot be queried.
#[must_use]
pub fn free_ram_mib() -> Option<u64> {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    Some(sys.free_memory() / (1024 * 1024))
}

/// Load-mode recommendation given free RAM and model size (both in MiB).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadAdvice {
    /// Requested mode is fine given current RAM.
    Ok,
    /// Switch to `StagedLoadOptions::resident()`: it fits and avoids refaults.
    PreferResident,
    /// Keep mmap but warn: free RAM is below the model size (refault risk).
    KeepMmap { deficit_mib: u64 },
    /// Resident/pinned requested but does not fit; mmap is the only option.
    FallbackToMmap,
}

/// Pure decision given injected sizes (MiB) and the requested staged mode.
#[must_use]
pub fn advise(free_mib: u64, model_mib: u64, opts: &StagedLoadOptions) -> LoadAdvice {
    if model_mib == 0 {
        return LoadAdvice::Ok;
    }
    let need_resident = model_mib + RESIDENT_HEADROOM_MIB;
    if opts.use_mmap {
        if !opts.use_mlock && free_mib >= need_resident {
            LoadAdvice::PreferResident
        } else if free_mib < model_mib {
            LoadAdvice::KeepMmap {
                deficit_mib: model_mib - free_mib,
            }
        } else {
            LoadAdvice::Ok
        }
    } else if free_mib < need_resident {
        LoadAdvice::FallbackToMmap
    } else {
        LoadAdvice::Ok
    }
}

/// Load mode for a low-RAM CPU box: `resident` when it fits, else `mmap`.
#[must_use]
pub fn low_ram_staged(free_mib: u64, model_mib: u64) -> StagedLoadOptions {
    match advise(free_mib, model_mib, &StagedLoadOptions::resident()) {
        LoadAdvice::FallbackToMmap => StagedLoadOptions::mmap(),
        _ => StagedLoadOptions::resident(),
    }
}

/// Human-readable warnings for a preflight run. Empty when everything is fine.
#[must_use]
pub fn warnings(
    free_mib: Option<u64>,
    model_mib: Option<u64>,
    opts: &StagedLoadOptions,
) -> Vec<String> {
    let mut out = Vec::new();
    let model_mib = match model_mib {
        Some(v) => v,
        None => {
            out.push("preflight: cannot read model file size (skipped)".to_string());
            return out;
        }
    };
    let free_mib = match free_mib {
        Some(v) => v,
        None => {
            out.push("preflight: cannot read free RAM (skipped)".to_string());
            return out;
        }
    };
    match advise(free_mib, model_mib, opts) {
        LoadAdvice::Ok => {}
        LoadAdvice::PreferResident => out.push(format!(
            "preflight: {free_mib} MiB free >= model {model_mib} MiB + {RESIDENT_HEADROOM_MIB} MiB headroom — use StagedLoadOptions::resident() (--no-mmap) to avoid mmap refaults"
        )),
        LoadAdvice::KeepMmap { deficit_mib } => out.push(format!(
            "preflight: {free_mib} MiB free is {deficit_mib} MiB below model size {model_mib} MiB — mmap pages will refault from disk (thrash). Close heavy apps or use --no-mmap; see docs/PERFORMANCE_RESEARCH.md"
        )),
        LoadAdvice::FallbackToMmap => out.push(format!(
            "preflight: resident/pinned needs {} MiB free but only {free_mib} MiB — falling back to mmap (thrash risk)",
            model_mib + RESIDENT_HEADROOM_MIB
        )),
    }
    if free_mib < CRITICAL_FREE_MIB {
        out.push(format!(
            "preflight: only {free_mib} MiB free — close other applications before inference"
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mmap_prefers_resident_when_it_fits() {
        let opts = StagedLoadOptions::mmap();
        assert_eq!(advise(8192, 6919, &opts), LoadAdvice::PreferResident);
    }

    #[test]
    fn mmap_keeps_mmap_with_deficit() {
        let opts = StagedLoadOptions::mmap();
        assert_eq!(
            advise(2048, 6919, &opts),
            LoadAdvice::KeepMmap { deficit_mib: 4871 }
        );
    }

    #[test]
    fn mmap_ok_until_resident_fits() {
        let opts = StagedLoadOptions::mmap();
        // free >= model (no refault) but < model + headroom (resident does not fit).
        assert_eq!(advise(2500, 2000, &opts), LoadAdvice::Ok);
        assert_eq!(advise(3023, 2000, &opts), LoadAdvice::Ok);
    }

    #[test]
    fn pinned_never_flips_to_resident() {
        let opts = StagedLoadOptions::pinned();
        assert_eq!(advise(8192, 6919, &opts), LoadAdvice::Ok);
    }

    #[test]
    fn resident_falls_back_when_short() {
        let opts = StagedLoadOptions::resident();
        assert_eq!(advise(3000, 6919, &opts), LoadAdvice::FallbackToMmap);
        assert_eq!(advise(8192, 6919, &opts), LoadAdvice::Ok);
    }

    #[test]
    fn low_ram_staged_picks_resident_or_mmap() {
        assert_eq!(low_ram_staged(8192, 6919), StagedLoadOptions::resident());
        assert_eq!(low_ram_staged(3000, 6919), StagedLoadOptions::mmap());
    }

    #[test]
    fn warnings_empty_when_ok() {
        let w = warnings(Some(2500), Some(2000), &StagedLoadOptions::mmap());
        assert!(w.is_empty());
    }

    #[test]
    fn warnings_report_deficit_and_critical_ram() {
        let w = warnings(Some(400), Some(6919), &StagedLoadOptions::mmap());
        assert!(w.iter().any(|s| s.contains("below model size")));
        assert!(w.iter().any(|s| s.contains("only 400 MiB free")));
    }

    #[test]
    fn warnings_skip_when_sizes_missing() {
        assert_eq!(
            warnings(None, Some(500), &StagedLoadOptions::mmap()).len(),
            1
        );
        assert_eq!(
            warnings(Some(1000), None, &StagedLoadOptions::mmap()).len(),
            1
        );
    }
}
