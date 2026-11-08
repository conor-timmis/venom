use time::OffsetDateTime;

use crate::{ChannelId, ChatUser, MessageId, Platform};

/// The service that owns an emote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmoteProvider {
    Twitch,
    YouTube,
    Kick,
    SevenTv,
    BetterTv,
    FrankerFaceZ,
}

impl EmoteProvider {
    /// Stable lowercase identifier used in asset keys and diagnostics.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Twitch => "twitch",
            Self::YouTube => "youtube",
            Self::Kick => "kick",
            Self::SevenTv => "7tv",
            Self::BetterTv => "bttv",
            Self::FrankerFaceZ => "ffz",
        }
    }

    /// Third-party providers are assets rather than streaming platforms
    /// (MAIN.md §7.7).
    #[must_use]
    pub const fn is_third_party(self) -> bool {
        matches!(self, Self::SevenTv | Self::BetterTv | Self::FrankerFaceZ)
    }
}

impl std::fmt::Display for EmoteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Structured message content.
///
/// Keeping segments instead of one painted string lets later phases render
/// badges and emotes inline without replacing the message model
/// (MAIN.md §6.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageSegment {
    Text(String),
    Emote {
        id: String,
        name: String,
        provider: EmoteProvider,
    },
    Link {
        label: String,
        url: String,
    },
    Mention {
        user_id: String,
        display_name: String,
    },
}

impl MessageSegment {
    /// Appends the plain-text representation used by the Phase 0 renderer.
    ///
    /// Rich rendering will consume the structured variants directly in
    /// Sprint 3; this helper is the lossy fallback until then.
    pub fn write_plain_text(&self, output: &mut String) {
        match self {
            Self::Text(text) => output.push_str(text),
            Self::Emote { name, .. } => output.push_str(name),
            Self::Link { label, .. } => output.push_str(label),
            Self::Mention { display_name, .. } => {
                output.push('@');
                output.push_str(display_name);
            }
        }
    }
}

/// Construction input for [`ChatMessage`].
///
/// A request struct keeps the constructor readable as the domain model grows
/// and avoids an eight-positional-parameter API.
#[derive(Debug, Clone)]
pub struct ChatMessageParts {
    pub id: MessageId,
    pub platform: Platform,
    pub channel_id: ChannelId,
    pub author: ChatUser,
    pub segments: Vec<MessageSegment>,
    pub timestamp: OffsetDateTime,
    pub received_at: OffsetDateTime,
    pub reply_to: Option<MessageId>,
}

/// A normalised chat message, independent of any platform payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    id: MessageId,
    platform: Platform,
    channel_id: ChannelId,
    author: ChatUser,
    segments: Vec<MessageSegment>,
    timestamp: OffsetDateTime,
    received_at: OffsetDateTime,
    reply_to: Option<MessageId>,
}

impl ChatMessage {
    pub fn new(parts: ChatMessageParts) -> Self {
        debug_assert!(
            !parts.segments.is_empty(),
            "chat messages must contain at least one segment"
        );

        Self {
            id: parts.id,
            platform: parts.platform,
            channel_id: parts.channel_id,
            author: parts.author,
            segments: parts.segments,
            timestamp: parts.timestamp,
            received_at: parts.received_at,
            reply_to: parts.reply_to,
        }
    }

    #[must_use]
    pub fn id(&self) -> &MessageId {
        &self.id
    }

    #[must_use]
    pub fn platform(&self) -> Platform {
        self.platform
    }

    #[must_use]
    pub fn channel_id(&self) -> &ChannelId {
        &self.channel_id
    }

    #[must_use]
    pub fn author(&self) -> &ChatUser {
        &self.author
    }

    #[must_use]
    pub fn segments(&self) -> &[MessageSegment] {
        &self.segments
    }

    /// Platform event time. UTC values are expected; display code converts to
    /// the user's local offset where needed.
    #[must_use]
    pub fn timestamp(&self) -> OffsetDateTime {
        self.timestamp
    }

    /// Local receipt time. Unified feeds order by this value (ADR-006).
    #[must_use]
    pub fn received_at(&self) -> OffsetDateTime {
        self.received_at
    }

    #[must_use]
    pub fn reply_to(&self) -> Option<&MessageId> {
        self.reply_to.as_ref()
    }

    /// Lossy plain-text projection used by the Phase 0 list prototype.
    #[must_use]
    pub fn plain_text(&self) -> String {
        let mut output = String::new();
        for segment in &self.segments {
            segment.write_plain_text(&mut output);
        }
        output.trim().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use time::{Duration, OffsetDateTime};

    use super::{ChatMessage, ChatMessageParts, EmoteProvider, MessageSegment};
    use crate::{ChannelId, ChatUser, MessageId, Platform, UserId};

    fn at(seconds: i64) -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH + Duration::seconds(seconds)
    }

    fn parts(segments: Vec<MessageSegment>) -> ChatMessageParts {
        ChatMessageParts {
            id: MessageId::new("twitch:channel:1"),
            platform: Platform::Twitch,
            channel_id: ChannelId::new("channel"),
            author: ChatUser::new(UserId::new("twitch:user:1"), "pixel"),
            segments,
            timestamp: at(10),
            received_at: at(11),
            reply_to: Some(MessageId::new("twitch:channel:0")),
        }
    }

    #[test]
    fn plain_text_renders_each_segment_kind() {
        let message = ChatMessage::new(parts(vec![
            MessageSegment::Text("hello ".to_owned()),
            MessageSegment::Emote {
                id: "twitch:Kappa".to_owned(),
                name: "Kappa".to_owned(),
                provider: EmoteProvider::Twitch,
            },
            MessageSegment::Text(" look ".to_owned()),
            MessageSegment::Link {
                label: "venom.cx".to_owned(),
                url: "https://venom.cx".to_owned(),
            },
            MessageSegment::Text(" ".to_owned()),
            MessageSegment::Mention {
                user_id: "youtube:user:2".to_owned(),
                display_name: "moth".to_owned(),
            },
        ]));

        assert_eq!(message.plain_text(), "hello Kappa look venom.cx @moth");
    }

    #[test]
    fn plain_text_trims_outer_whitespace() {
        let message = ChatMessage::new(parts(vec![MessageSegment::Text(
            "  padded message  ".to_owned(),
        )]));

        assert_eq!(message.plain_text(), "padded message");
    }

    #[test]
    fn timestamps_and_reply_identity_are_preserved() {
        let message = ChatMessage::new(parts(vec![MessageSegment::Text("hi".to_owned())]));

        assert_eq!(message.timestamp(), at(10));
        assert_eq!(message.received_at(), at(11));
        assert_eq!(
            message.reply_to().map(MessageId::as_str),
            Some("twitch:channel:0")
        );
    }
}
