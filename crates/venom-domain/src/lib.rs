//! Platform-independent domain types shared by Venom's adapters, core, and UI.
//!
//! This crate deliberately has no dependency on Slint, SQLite, Tokio, or any
//! streaming-platform SDK. Identity values are scoped to a [`Platform`]: a
//! Twitch user and a YouTube user with the same display name are not the same
//! person (MAIN.md §6.5).
//!
//! Phase 0 uses these types for the synthetic rendering prototype. Later phases
//! add real adapters and storage on top of the same contracts.

mod channel;
mod ids;
mod message;
mod platform;
mod user;

pub use channel::Channel;
pub use ids::{ChannelId, MessageId, UserId};
pub use message::{ChatMessage, ChatMessageParts, EmoteProvider, MessageSegment};
pub use platform::Platform;
pub use user::ChatUser;
