//! Conversion from domain messages to Slint-facing row values.

use slint::{Color, ModelRc, SharedString, VecModel};
use venom_domain::{BadgeKind, ChatMessage, ChatUser, MessageSegment, Platform};

use super::{author_color, layout};
use crate::{
    ChatBadge, ChatBadgeKind, ChatLine, ChatRow, ChatSegment, ChatSegmentKind, assets::AssetStore,
    format::Clock,
};

const REPLY_EXCERPT_CHARS: usize = 80;
const ELLIPSIS: char = '\u{2026}';

/// Ambient values shared by every row in one mapping pass.
pub struct RowContext<'a> {
    pub clock: Clock,
    pub body_width: f32,
    pub assets: &'a mut AssetStore,
}

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
pub fn to_chat_row(
    message: &ChatMessage,
    reply: Option<&ReplySummary>,
    context: &mut RowContext<'_>,
) -> ChatRow {
    ChatRow {
        platform_label: SharedString::from(platform_label(message.platform())),
        platform_accent: platform_accent(message.platform()),
        author: SharedString::from(message.author().display_name.as_str()),
        author_color: author_color::for_name(&message.author().display_name),
        badges: to_badges(message.author()),
        timestamp: SharedString::from(context.clock.format(message.timestamp())),
        body: to_body(message.segments(), context),
        reply_preview: SharedString::from(format_reply(reply)),
    }
}

fn to_badges(user: &ChatUser) -> ModelRc<ChatBadge> {
    let badges = user
        .badges()
        .iter()
        .map(|badge| ChatBadge {
            label: SharedString::from(badge.label()),
            kind: to_badge_kind(badge.kind()),
        })
        .collect::<Vec<_>>();

    ModelRc::new(VecModel::from(badges))
}

fn to_body(segments: &[MessageSegment], context: &mut RowContext<'_>) -> ModelRc<ChatLine> {
    let lines = layout::wrap(segments, context.body_width)
        .into_iter()
        .map(|line| ChatLine {
            segments: ModelRc::new(VecModel::from(
                line.segments()
                    .iter()
                    .map(|inline| to_segment(inline, context.assets))
                    .collect::<Vec<_>>(),
            )),
        })
        .collect::<Vec<_>>();

    ModelRc::new(VecModel::from(lines))
}

fn to_segment(inline: &layout::Inline, assets: &mut AssetStore) -> ChatSegment {
    match inline {
        layout::Inline::Text(text) => plain_segment(ChatSegmentKind::Text, text.clone()),
        layout::Inline::Link { label, .. } => plain_segment(ChatSegmentKind::Link, label.clone()),
        layout::Inline::Mention { display_name, .. } => {
            plain_segment(ChatSegmentKind::Mention, layout::mention_text(display_name))
        }
        layout::Inline::Emote { id, name, provider } => {
            let asset = assets.emote(*provider, id);
            ChatSegment {
                kind: ChatSegmentKind::Emote,
                text: SharedString::from(name.as_str()),
                frames: asset.frames(),
                frame_delay: i32::from(asset.frame_delay_ms()),
            }
        }
    }
}

fn plain_segment(kind: ChatSegmentKind, text: String) -> ChatSegment {
    ChatSegment {
        kind,
        text: SharedString::from(text),
        frames: ModelRc::default(),
        frame_delay: 0,
    }
}

fn to_badge_kind(kind: BadgeKind) -> ChatBadgeKind {
    match kind {
        BadgeKind::Broadcaster => ChatBadgeKind::Broadcaster,
        BadgeKind::Moderator => ChatBadgeKind::Moderator,
        BadgeKind::Vip => ChatBadgeKind::Vip,
        BadgeKind::Subscriber => ChatBadgeKind::Subscriber,
        BadgeKind::Prime => ChatBadgeKind::Prime,
        BadgeKind::Staff => ChatBadgeKind::Staff,
        BadgeKind::Other => ChatBadgeKind::Other,
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
    use slint::Model;
    use time::{Duration, OffsetDateTime, UtcOffset};
    use venom_domain::{
        BadgeKind, ChannelId, ChatMessage, ChatMessageParts, ChatUser, EmoteProvider, MessageBadge,
        MessageId, MessageSegment, Platform, UserId,
    };

    use super::{
        ReplySummary, RowContext, excerpt, platform_accent, platform_label, to_badge_kind,
        to_chat_row,
    };
    use crate::{
        ChatBadgeKind, ChatSegmentKind,
        assets::{ANIMATED_FRAME_COUNT, AssetStore, DEFAULT_CACHE_CAPACITY},
        format::Clock,
        ui_bridge::author_color,
    };

    fn at(seconds: i64) -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH + Duration::seconds(seconds)
    }

    fn sample_message() -> ChatMessage {
        ChatMessage::new(ChatMessageParts {
            id: MessageId::new("twitch:channel:1"),
            platform: Platform::Twitch,
            channel_id: ChannelId::new("channel"),
            author: ChatUser::new(UserId::new("twitch:user:1"), "moonlit_mara").with_badges(vec![
                MessageBadge::new("twitch:moderator", "MOD", BadgeKind::Moderator),
                MessageBadge::new("twitch:subscriber", "SUB", BadgeKind::Subscriber),
            ]),
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

    fn context(assets: &mut AssetStore) -> RowContext<'_> {
        RowContext {
            clock: Clock::fixed(UtcOffset::UTC),
            body_width: super::layout::DEFAULT_BODY_WIDTH_PX,
            assets,
        }
    }

    fn line_texts(row: &crate::ChatRow) -> Vec<String> {
        (0..row.body.row_count())
            .map(|index| {
                let line = row.body.row_data(index).expect("line");
                (0..line.segments.row_count())
                    .map(|segment| {
                        line.segments
                            .row_data(segment)
                            .expect("segment")
                            .text
                            .to_string()
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn maps_identity_content_and_timestamp() {
        let mut assets = AssetStore::new(DEFAULT_CACHE_CAPACITY);
        let row = to_chat_row(&sample_message(), None, &mut context(&mut assets));

        assert_eq!(row.platform_label.as_str(), "TWITCH");
        assert_eq!(row.author.as_str(), "moonlit_mara");
        assert_eq!(row.timestamp.as_str(), "00:00:01");
        assert_eq!(line_texts(&row), ["hello Kappa"]);
        assert!(row.reply_preview.is_empty());
    }

    #[test]
    fn maps_badges_in_display_order() {
        let mut assets = AssetStore::new(DEFAULT_CACHE_CAPACITY);
        let row = to_chat_row(&sample_message(), None, &mut context(&mut assets));

        assert_eq!(row.badges.row_count(), 2);
        let first = row.badges.row_data(0).expect("first badge");
        let second = row.badges.row_data(1).expect("second badge");
        assert_eq!(first.label.as_str(), "MOD");
        assert_eq!(first.kind, ChatBadgeKind::Moderator);
        assert_eq!(second.label.as_str(), "SUB");
        assert_eq!(second.kind, ChatBadgeKind::Subscriber);
    }

    #[test]
    fn maps_emotes_to_cached_image_frames() {
        let mut assets = AssetStore::new(DEFAULT_CACHE_CAPACITY);
        let row = to_chat_row(&sample_message(), None, &mut context(&mut assets));
        let line = row.body.row_data(0).expect("line");
        let emote = line.segments.row_data(1).expect("emote");

        assert_eq!(emote.kind, ChatSegmentKind::Emote);
        assert_eq!(emote.frames.row_count(), 1);
        assert_eq!(assets.stats().entries, 1);
    }

    #[test]
    fn animated_emotes_carry_a_frame_sequence() {
        let message = ChatMessage::new(ChatMessageParts {
            id: MessageId::new("twitch:channel:2"),
            platform: Platform::Twitch,
            channel_id: ChannelId::new("channel"),
            author: ChatUser::new(UserId::new("twitch:user:1"), "moonlit_mara"),
            segments: vec![MessageSegment::Emote {
                id: "7tv:catJAM".to_owned(),
                name: "catJAM".to_owned(),
                provider: EmoteProvider::SevenTv,
            }],
            timestamp: at(1),
            received_at: at(1),
            reply_to: None,
        });

        let mut assets = AssetStore::new(DEFAULT_CACHE_CAPACITY);
        let row = to_chat_row(&message, None, &mut context(&mut assets));
        let line = row.body.row_data(0).expect("line");
        let emote = line.segments.row_data(0).expect("emote");

        assert_eq!(emote.frames.row_count(), ANIMATED_FRAME_COUNT);
        assert!(emote.frame_delay > 0);
    }

    #[test]
    fn maps_links_and_mentions_with_their_render_text() {
        let message = ChatMessage::new(ChatMessageParts {
            id: MessageId::new("twitch:channel:3"),
            platform: Platform::Twitch,
            channel_id: ChannelId::new("channel"),
            author: ChatUser::new(UserId::new("twitch:user:1"), "moonlit_mara"),
            segments: vec![
                MessageSegment::Text("see ".to_owned()),
                MessageSegment::Link {
                    label: "the clip".to_owned(),
                    url: "https://example.test/clip".to_owned(),
                },
                MessageSegment::Text(" and ".to_owned()),
                MessageSegment::Mention {
                    user_id: "twitch:user:2".to_owned(),
                    display_name: "moth".to_owned(),
                },
            ],
            timestamp: at(1),
            received_at: at(1),
            reply_to: None,
        });

        let mut assets = AssetStore::new(DEFAULT_CACHE_CAPACITY);
        let row = to_chat_row(&message, None, &mut context(&mut assets));
        let line = row.body.row_data(0).expect("line");

        assert_eq!(
            line.segments.row_data(1).expect("link").kind,
            ChatSegmentKind::Link
        );
        assert_eq!(
            line.segments.row_data(3).expect("mention").kind,
            ChatSegmentKind::Mention
        );
        assert_eq!(line_texts(&row), ["see the clip and @moth"]);
    }

    #[test]
    fn maps_reply_preview_and_empty_reply_is_blank() {
        let message = sample_message();
        let preview = ReplySummary::from_message(&sample_message());
        let mut assets = AssetStore::new(DEFAULT_CACHE_CAPACITY);
        let row = to_chat_row(&message, Some(&preview), &mut context(&mut assets));

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
        assert_ne!(twitch, slint::Color::default());
    }

    #[test]
    fn platform_labels_are_stable() {
        assert_eq!(platform_label(Platform::Twitch), "TWITCH");
        assert_eq!(platform_label(Platform::YouTube), "YOUTUBE");
        assert_eq!(platform_label(Platform::Kick), "KICK");
    }

    #[test]
    fn badge_kinds_map_to_the_ui_enum() {
        assert_eq!(
            to_badge_kind(BadgeKind::Broadcaster),
            ChatBadgeKind::Broadcaster
        );
        assert_eq!(
            to_badge_kind(BadgeKind::Moderator),
            ChatBadgeKind::Moderator
        );
        assert_eq!(to_badge_kind(BadgeKind::Vip), ChatBadgeKind::Vip);
        assert_eq!(
            to_badge_kind(BadgeKind::Subscriber),
            ChatBadgeKind::Subscriber
        );
        assert_eq!(to_badge_kind(BadgeKind::Prime), ChatBadgeKind::Prime);
        assert_eq!(to_badge_kind(BadgeKind::Staff), ChatBadgeKind::Staff);
        assert_eq!(to_badge_kind(BadgeKind::Other), ChatBadgeKind::Other);
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
