# llama-cpp-2 0.1.156 ← llama-cpp-sys-2 0.1.161 API adaption

## Why

llama-cpp-2 0.1.156 (the safe Rust wrapper pin) predates the sys-0.1.161
llama.cpp tree, whose C++ wrapper `llama_rs_fit_params` (wrapper_common.h/cpp →
`common_fit_params`) gained a new parameter:

    const void * extra   // const common_fit_extra_model * — model fitted
                         // alongside the main one; nullptr = fit alone

The 0.1.156 wrapper `src/model/params.rs` still calls it with 8 arguments
(E0061), while the generated bindings require 9.

## What

`src/model/params.rs` (fit function): pass `std::ptr::null::<c_void>()` as the
new `extra` argument. Upstream explicitly supports `extra == nullptr`
(fit.cpp: `if (extra == nullptr) { return; }` — the extra model is skipped and
only the main model is fitted). Draft/extra-model fit (MTP) is a 0.1.157+
feature and stays out of scope here.

## Patch (unified diff)

~~~diff
--- a/src/model/params.rs
+++ b/src/model/params.rs
@@ -401,7 +401,12 @@
                 self.buft_overrides.as_mut_ptr(),
                 margins.as_mut_ptr(),
                 n_ctx_min,
+                // sys 0.1.161 added common_fit_extra_model (extra model fitted
+                // alongside the main one; 0.1.156 wrapper knows no draft-fit API).
+                // nullptr is explicitly supported by common_fit_params: the extra
+                // model is skipped and only the main model is fitted (upstream way).
+                std::ptr::null::<std::os::raw::c_void>(),
                 log_level,
             )
         };
~~~

##ượng Apply

    cd ~/.cargo/registry/src/index.crates.io-*/llama-cpp-2-0.1.156
    patch -p1 -F2 < patches/llama-cpp-2-0.1.156/fit_extra_null.patch

Re-apply after every registry refresh of this crate version (same law as the
six llama-cpp-sys-2-0.1.154 → 0.1.161 patches).

## llama-cpp-sys-2 0.1.161 (re-applied 2026-11-06)

- `build.rs.patch` — applied earlier in the B-3 phase (verified: buildinfo TU,
  llama-common-base static link, cpp-httplib link, WindowsVariant::Other
  advapi32/shell32/ws2_32).
- `log.cpp.patch` — re-applied clean (`patch -p1`, hunk offset +79).
- `speculative.cpp.patch` — hunk #1 re-applied (offset +77, fuzz 1); hunk #2
  applied manually (upstream moved the `ctx_dft pos_max < N-1` warning block
  into `common_speculative_impl_draft_mtp::begin`, removed there by hand).
- vendored `ggml-rpc` proto-5.0.0 sources — NOT re-applied yet (copying an
  older proto into the 0.1.161 tree risks a version mismatch; re-copy from
  `patches/llama-cpp-sys-2-0.1.154/ggml-rpc/` only when GGML_RPC=ON rpc builds
  are claimed, and verify the wire protocol at runtime).
