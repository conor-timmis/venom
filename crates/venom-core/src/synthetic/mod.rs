//! Deterministic, platform-independent synthetic chat.

mod config;
mod content;
mod generator;
mod names;
mod random;

pub use config::{
    DEFAULT_SEED, SyntheticConfig, SyntheticConfigError, default_channels, default_start_time,
};
pub use generator::SyntheticChatGenerator;
