//! Platform-independent processing for Venom.
//!
//! The first deliverable is the deterministic synthetic chat generator used by
//! the Phase 0 rendering prototype. It lives here rather than in the app so
//! rendering, routing, and later performance benchmarks share one message
//! source.

pub mod synthetic;

pub use synthetic::{
    DEFAULT_SEED, SyntheticChatGenerator, SyntheticConfig, SyntheticConfigError, default_channels,
    default_start_time,
};
