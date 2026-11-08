//! Conversion from domain messages to Slint-facing row values.

use slint::{Color, SharedString};
use venom_domain::{ChatMessage, Platform};

use super::author_color;
use crate::{ChatRow, format::Clock};

const REPLY_EXCERPT_CHARS: usize = 80;
const ELLIPSIS: char = '\u{2026}';

/// The excerpt shown when a message replies to another retained message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplySummary {
    author: String,
    excerpt: String,
}

impl ReplySummary {
    #[must_use]
    pub fn from_message(message: &ChatMessage) -> Self {
        Self {
            author: message.author().display_name.clone(),
            excerpt: excerpt(&message.plain_text()),
        }
    }

    #[must_use]
    pub fn author(&self) -> &str {
        &self.author
    }
}

/// Maps one normalised message to the immutable row Slint renders.
#[must_use]
pub fn to_chat_row(message: &ChatMessage, reply: Option<&ReplySummary>, clock: Clock) -> ChatRow {
    ChatRow {
        platform_label: SharedString::from(platform_label(message.platform())),
        platform_accent: platform_accent(message.platform()),
        author: SharedString::from(message.author().display_name.as_str()),
        author_color: author_color::for_name(&message.author().display_name),
        timestamp: SharedString::from(clock.format(message.timestamp())),
        body: SharedString::from(message.plain_text()),
        reply_preview: SharedString::from(format_reply(reply)),
    }
}

fn platform_label(platform: Platform) -> &'static str {
    match platform {
        Platform::Twitch => "TWITCH",
        Platform::YouTube => "YOUTUBE",
        Platform::Kick => "KICK",
    }
}

fn platform_accent(platform: Platform) -> Color {
    match platform {
        Platform::Twitch => Color::from_rgb_u8(0x9d, 0x6b, 0xff),
        Platform::YouTube => Color::from_rgb_u8(0xff, 0x4d, 0x4d),
        Platform::Kick => Color::from_rgb_u8(0x53, 0xfc, 0x18),
    }
}

fn format_reply(reply: Option<&ReplySummary>) -> String {
    match reply {
        Some(reply) => format!("\u{21b3} {}: {}", reply.author(), reply.excerpt),
        None => String::new(),
    }
}

fn excerpt(text: &str) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= REPLY_EXCERPT_CHARS {
        return normalized;
    }

    let mut shortened = normalized
        .chars()
        .take(REPLY_EXCERPT_CHARS)
        .collect::<String>();
    shortened.push(ELLIPSIS);
    shortened
}

#[cfg(test)]
mod tests {
    use slint::Color;
    use time::{Duration, OffsetDateTime, UtcOffset};
    use venom_domain::{
        ChannelId, ChatMessage, ChatMessageParts, ChatUser, EmoteProvider, MessageId,
        MessageSegment, Platform, UserId,
    };

    use super::{ReplySummary, excerpt, platform_accent, platform_label, to_chat_row};
    use crate::format::Clock;
    use crate::ui_bridge::author_color;

    fn at(seconds: i64) -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH + Duration::seconds(seconds)
    }

    fn sample_message() -> ChatMessage {
        ChatMessage::new(ChatMessageParts {
            id: MessageId::new("twitch:channel:1"),
            platform: Platform::Twitch,
            channel_id: ChannelId::new("channel"),
            author: ChatUser::new(UserId::new("twitch:user:1"), "moonlit_mara"),
            segments: vec![
                MessageSegment::Text("hello ".to_owned()),
                MessageSegment::Emote {
                    id: "twitch:Kappa".to_owned(),
                    name: "Kappa".to_owned(),
                    provider: EmoteProvider::Twitch,
                },
            ],
            timestamp: at(1),
            received_at: at(1),
            reply_to: None,
        })
    }

    #[test]
    fn maps_identity_content_and_timestamp() {
        let clock = Clock::fixed(UtcOffset::UTC);
        let row = to_chat_row(&sample_message(), None, clock);

        assert_eq!(row.platform_label.as_str(), "TWITCH");
        assert_eq!(row.author.as_str(), "moonlit_mara");
        assert_eq!(row.timestamp.as_str(), "00:00:01");
        assert_eq!(row.body.as_str(), "hello Kappa");
        assert!(row.reply_preview.is_empty());
    }

    #[test]
    fn maps_reply_preview_and_empty_reply_is_blank() {
        let message = sample_message();
        let preview = ReplySummary::from_message(&sample_message());
        let clock = Clock::fixed(UtcOffset::UTC);
        let row = to_chat_row(&message, Some(&preview), clock);

        assert_eq!(
            row.reply_preview.as_str(),
            "\u{21b3} moonlit_mara: hello Kappa"
        );
    }

    #[test]
    fn platform_accents_are_distinct() {
        let twitch = platform_accent(Platform::Twitch);
        let youtube = platform_accent(Platform::YouTube);
        let kick = platform_accent(Platform::Kick);

        assert_ne!(twitch, youtube);
        assert_ne!(youtube, kick);
        assert_ne!(twitch, kick);
        assert_ne!(twitch, Color::default());
    }

    #[test]
    fn platform_labels_are_stable() {
        assert_eq!(platform_label(Platform::Twitch), "TWITCH");
        assert_eq!(platform_label(Platform::YouTube), "YOUTUBE");
        assert_eq!(platform_label(Platform::Kick), "KICK");
    }

    #[test]
    fn author_colours_are_stable() {
        assert_eq!(
            author_color::for_name("moonlit_mara"),
            author_color::for_name("moonlit_mara")
        );
        assert_ne!(
            author_color::for_name("moonlit_mara"),
            author_color::for_name("pixelpilot")
        );
    }

    #[test]
    fn excerpts_are_normalised_and_truncated() {
        assert_eq!(excerpt("  hello\n  chat  "), "hello chat");

        let long = "x".repeat(200);
        let shortened = excerpt(&long);
        assert_eq!(shortened.chars().count(), 81);
        assert!(shortened.ends_with('\u{2026}'));
    }
}
