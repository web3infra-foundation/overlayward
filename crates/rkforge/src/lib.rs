//! rkforge — OCI image build/push/pull library.
//! Fork of upstream rkforge (rk8s), adapted for Overlayward workspace.
//!
//! Module activation by feature gate:
//! - default: pull/storage/registry/images/config (M2 core)
//! - "build": image/ + compressor/ + push/ + task/run/copy (DS-2+)
//! - "overlayfs": overlayfs/ (DS-2+, needs libfuse-fs)
//! - "sandbox": sandbox/ (M3b, Firecracker MicroVM)

// ── M2 core (default) ──
pub mod pull;
pub mod storage;
pub mod registry;
pub mod images;
pub mod config;
pub mod oci_spec;
pub(crate) mod login;
pub(crate) mod logout;
pub(crate) mod repo;
pub(crate) mod rt;
pub(crate) mod utils;

// ── DS-2+ (build feature) ──
#[cfg(feature = "build")]
pub mod image;
#[cfg(feature = "build")]
pub(crate) mod compressor;
#[cfg(feature = "build")]
pub mod push;
#[cfg(feature = "build")]
pub(crate) mod task;
#[cfg(feature = "build")]
pub(crate) mod run;
#[cfg(feature = "build")]
pub(crate) mod copy;

#[cfg(feature = "overlayfs")]
pub mod overlayfs;

// ── M3b (sandbox feature) ──
#[cfg(feature = "sandbox")]
pub mod sandbox;

// ── re-exports (conditional) ──
#[cfg(feature = "build")]
pub use image::build_runtime;

// ── Not compiled (kept in source tree for reference) ──
// commands/   — rk8s CLI entry points, not reused
// pod_task.rs — multi-container orchestration, Phase 4 M11 reference
// args.rs, main.rs — not compiled in lib mode
