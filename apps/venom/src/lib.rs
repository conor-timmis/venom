//! Venom desktop library.
//!
//! The binary entrypoint stays deliberately thin: startup, composition, and
//! rendering live in focused modules so the Phase 0 prototype can grow without
//! turning `main.rs` into a dumping ground.

/// Slint-generated UI module.
///
/// The macro emits `todo!` stubs for Rust-authored components and scoped
/// `allow(unsafe_code)` blocks for its FFI glue, so the production-code lint
/// denials must be relaxed only around the generated module.
#[allow(clippy::todo, clippy::unimplemented)]
mod generated_ui {
    slint::include_modules!();
}

pub use generated_ui::*;

mod app;
mod config;
mod feed;
mod format;
mod logging;
mod metrics;
mod ui_bridge;

pub use app::{AppError, run};
