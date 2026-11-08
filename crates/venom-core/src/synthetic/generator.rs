//! Stateful synthetic chat generation.
//!
//! The generator owns all mutable state needed for a replayable stream: the
//! random source, the synthetic clock, platform-scoped message identities, and
//! a short per-channel reply history. It intentionally has no knowledge of
//! Slint, storage, or live platform payloads.

use std::collections::VecDeque;

use fastrand::Rng;
use time::{Duration, OffsetDateTime};
use venom_domain::{
    BadgeKind, Channel, ChannelId, ChatMessage, ChatMessageParts, ChatUser, MessageBadge,
    MessageId, MessageSegment, Platform, UserId,
};

use super::{
    SyntheticConfig, content, names,
    random::{chance, pick_index, weighted_index},
};

const REPLY_PERCENT: u32 = 12;
const REPLY_HISTORY: usize = 128;
const MIN_STEP_SECONDS: u32 = 1;
const MAX_STEP_SECONDS: u32 = 8;
const STEP_RANGE_SECONDS: u32 = MAX_STEP_SECONDS - MIN_STEP_SECONDS + 1;
const RECEIPT_DELAY_MILLIS: u32 = 24;

const TWITCH_WEIGHT: u32 = 55;
const YOUTUBE_WEIGHT: u32 = 25;
const KICK_WEIGHT: u32 = 20;

/// Deterministic source of normalised chat messages.
#[derive(Debug)]
pub struct SyntheticChatGenerator {
    seed: u64,
    rng: Rng,
    start_time: OffsetDateTime,
    clock: OffsetDateTime,
    channels: Vec<ChannelFeed>,
    channel_weights: Vec<u32>,
    total_channel_weight: u32,
    next_sequence: u64,
}

impl SyntheticChatGenerator {
    #[must_use]
    pub fn new(config: SyntheticConfig) -> Self {
        let (seed, channels, start_time) = config.into_parts();
        let channel_weights: Vec<u32> = channels
            .iter()
            .map(|channel| channel_weight(channel.platform))
            .collect();
        let total_channel_weight = channel_weights.iter().sum();

        Self {
            seed,
            rng: Rng::with_seed(seed),
            start_time,
            clock: start_time,
            channels: channels.into_iter().map(ChannelFeed::new).collect(),
            channel_weights,
            total_channel_weight,
            next_sequence: 1,
        }
    }

    /// Restores the generator to its configured starting state.
    pub fn reset(&mut self) {
        self.rng = Rng::with_seed(self.seed);
        self.clock = self.start_time;
        self.next_sequence = 1;

        for channel in &mut self.channels {
            channel.recent_ids.clear();
        }
    }

    pub fn next_message(&mut self) -> ChatMessage {
        let channel_index = self.pick_channel_index();
        let pending = self.compose_message(channel_index);

        let timestamp = self.clock;
        let step = Duration::seconds(i64::from(
            MIN_STEP_SECONDS + self.rng.u32(..STEP_RANGE_SECONDS),
        ));
        self.clock += step;
        let received_at =
            timestamp + Duration::milliseconds(i64::from(self.rng.u32(..RECEIPT_DELAY_MILLIS)));

        let id = self.next_message_id(pending.platform, &pending.channel_id);
        let message = ChatMessage::new(ChatMessageParts {
            id: id.clone(),
            platform: pending.platform,
            channel_id: pending.channel_id,
            author: pending.author,
            segments: pending.segments,
            timestamp,
            received_at,
            reply_to: pending.reply_to,
        });

        self.remember_for_replies(channel_index, id);
        message
    }

    pub fn take(&mut self, count: usize) -> Vec<ChatMessage> {
        let mut messages = Vec::with_capacity(count);

        for _ in 0..count {
            messages.push(self.next_message());
        }

        messages
    }

    fn compose_message(&mut self, channel_index: usize) -> PendingMessage {
        let channel = &self.channels[channel_index];
        let author_index = pick_index(&mut self.rng, channel.authors.len());
        let author = channel.authors[author_index].clone();
        let segments = content::compose(&mut self.rng, &author, &channel.authors);
        let reply_to = choose_reply(&mut self.rng, &channel.recent_ids);

        PendingMessage {
            platform: channel.channel.platform,
            channel_id: channel.channel.id.clone(),
            author,
            segments,
            reply_to,
        }
    }

    fn next_message_id(&mut self, platform: Platform, channel_id: &ChannelId) -> MessageId {
        let sequence = self.next_sequence;
        self.next_sequence += 1;

        MessageId::new(format!(
            "{}:{}:{}",
            platform.as_str(),
            channel_id.as_str(),
            sequence
        ))
    }

    fn remember_for_replies(&mut self, channel_index: usize, message_id: MessageId) {
        let recent_ids = &mut self.channels[channel_index].recent_ids;
        recent_ids.push_back(message_id);

        while recent_ids.len() > REPLY_HISTORY {
            recent_ids.pop_front();
        }
    }

    fn pick_channel_index(&mut self) -> usize {
        weighted_index(
            &mut self.rng,
            &self.channel_weights,
            self.total_channel_weight,
        )
    }
}

struct PendingMessage {
    platform: Platform,
    channel_id: ChannelId,
    author: ChatUser,
    segments: Vec<MessageSegment>,
    reply_to: Option<MessageId>,
}

#[derive(Debug)]
struct ChannelFeed {
    channel: Channel,
    authors: Vec<ChatUser>,
    recent_ids: VecDeque<MessageId>,
}

impl ChannelFeed {
    fn new(channel: Channel) -> Self {
        let authors = names::USER_NAMES
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let user_id = format!(
                    "{}:{}:user:{index}",
                    channel.platform.as_str(),
                    channel.id.as_str()
                );
                ChatUser::new(UserId::new(user_id), *name)
                    .with_badges(badges_for(channel.platform, index))
            })
            .collect();

        Self {
            channel,
            authors,
            recent_ids: VecDeque::with_capacity(REPLY_HISTORY),
        }
    }
}

/// Assigns a deterministic, varied badge set to a synthetic author.
///
/// The rules are deliberately spread across the roster so every badge kind
/// appears without needing a much larger author pool.
fn badges_for(platform: Platform, index: usize) -> Vec<MessageBadge> {
    names::BADGES
        .iter()
        .filter(|spec| match spec.kind {
            BadgeKind::Broadcaster => index == 0,
            BadgeKind::Moderator => index != 0 && index.is_multiple_of(6),
            BadgeKind::Vip => index % 5 == 2,
            BadgeKind::Subscriber => index.is_multiple_of(3),
            BadgeKind::Prime => index % 13 == 4,
            BadgeKind::Staff => index % 19 == 8,
            BadgeKind::Other => false,
        })
        .map(|spec| {
            MessageBadge::new(
                format!("{}:{}", platform.as_str(), spec.id),
                spec.label,
                spec.kind,
            )
        })
        .collect()
}

fn choose_reply(rng: &mut Rng, recent_ids: &VecDeque<MessageId>) -> Option<MessageId> {
    if recent_ids.is_empty() || !chance(rng, REPLY_PERCENT) {
        return None;
    }

    let index = pick_index(rng, recent_ids.len());
    recent_ids.get(index).cloned()
}

const fn channel_weight(platform: Platform) -> u32 {
    match platform {
        Platform::Twitch => TWITCH_WEIGHT,
        Platform::YouTube => YOUTUBE_WEIGHT,
        Platform::Kick => KICK_WEIGHT,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::SyntheticChatGenerator;
    use crate::synthetic::SyntheticConfig;
    use venom_domain::{BadgeKind, Channel, Platform};

    fn generator(seed: u64) -> SyntheticChatGenerator {
        let config = SyntheticConfig::new(seed, crate::synthetic::default_channels())
            .expect("default channels are present");
        SyntheticChatGenerator::new(config)
    }

    #[test]
    fn same_seed_replays_an_identical_stream() {
        let first = generator(42).take(500);
        let second = generator(42).take(500);

        assert_eq!(first, second);
    }

    #[test]
    fn different_seeds_produce_different_content() {
        let first = generator(1).take(200);
        let second = generator(2).take(200);

        assert_ne!(first, second);
    }

    #[test]
    fn reset_returns_to_the_start_of_the_stream() {
        let mut generator = generator(7);
        let first = generator.take(100);
        generator.reset();

        assert_eq!(first, generator.take(100));
    }

    #[test]
    fn first_timestamp_uses_the_configured_origin() {
        let start_time = time::OffsetDateTime::UNIX_EPOCH + time::Duration::hours(9);
        let config = SyntheticConfig::default().with_start_time(start_time);
        let mut generator = SyntheticChatGenerator::new(config);

        assert_eq!(generator.next_message().timestamp(), start_time);
    }

    #[test]
    fn timestamps_and_receipt_times_never_move_backwards() {
        let messages = generator(3).take(2_000);

        for pair in messages.windows(2) {
            assert!(pair[0].timestamp() <= pair[1].timestamp());
            assert!(pair[0].received_at() <= pair[1].received_at());
            assert!(pair[0].received_at() >= pair[0].timestamp());
        }
    }

    #[test]
    fn all_configured_platforms_appear() {
        let messages = generator(5).take(5_000);
        let platforms: HashSet<_> = messages.iter().map(|message| message.platform()).collect();

        assert_eq!(platforms, Platform::ALL.into_iter().collect());
    }

    #[test]
    fn message_ids_are_unique_and_platform_scoped() {
        let messages = generator(11).take(10_000);
        let identifiers: HashSet<_> = messages
            .iter()
            .map(|message| message.id().as_str())
            .collect();

        assert_eq!(identifiers.len(), messages.len());
        assert!(messages.iter().all(|message| {
            message
                .id()
                .as_str()
                .starts_with(message.platform().as_str())
        }));
    }

    #[test]
    fn authors_are_scoped_to_their_platform() {
        let messages = generator(13).take(2_000);

        assert!(messages.iter().all(|message| {
            message
                .author()
                .id
                .as_str()
                .starts_with(message.platform().as_str())
        }));
    }

    #[test]
    fn reply_targets_reference_earlier_messages_in_the_same_channel() {
        let messages = generator(17).take(5_000);
        let mut seen = HashSet::new();
        let mut channel_by_id = HashMap::new();
        let mut replies = 0;

        for message in &messages {
            if let Some(reply_to) = message.reply_to() {
                replies += 1;
                assert!(
                    seen.contains(reply_to.as_str()),
                    "reply target must be earlier"
                );
                assert_eq!(
                    channel_by_id.get(reply_to.as_str()),
                    Some(&message.channel_id()),
                    "replies stay in the same channel"
                );
            }

            seen.insert(message.id().as_str().to_owned());
            channel_by_id.insert(message.id().as_str().to_owned(), message.channel_id());
        }

        assert!(replies > 0, "the stream should contain reply examples");
    }

    #[test]
    fn every_message_has_plain_text_and_varied_length() {
        let messages = generator(19).take(2_000);
        let lengths: Vec<_> = messages
            .iter()
            .map(|message| message.plain_text().chars().count())
            .collect();

        assert!(
            messages
                .iter()
                .all(|message| !message.plain_text().is_empty())
        );
        assert!(lengths.iter().any(|length| *length < 40));
        assert!(lengths.iter().any(|length| *length > 160));
    }

    #[test]
    fn single_channel_configuration_never_switches_channels() {
        let channel = Channel::new(Platform::Kick, "only-channel", "Only Channel");
        let config = SyntheticConfig::new(23, vec![channel]).expect("one channel is present");
        let mut generator = SyntheticChatGenerator::new(config);
        let messages = generator.take(200);

        assert!(
            messages
                .iter()
                .all(|message| message.channel_id().as_str() == "only-channel")
        );
    }

    #[test]
    fn take_zero_returns_an_empty_batch() {
        assert!(generator(29).take(0).is_empty());
    }

    #[test]
    fn authors_carry_platform_scoped_badges() {
        let messages = generator(31).take(5_000);

        assert!(
            messages
                .iter()
                .any(|message| !message.author().badges().is_empty())
        );
        assert!(messages.iter().all(|message| {
            message.author().badges().iter().all(|badge| {
                badge.id().starts_with(message.platform().as_str()) && !badge.label().is_empty()
            })
        }));
    }

    #[test]
    fn every_synthetic_badge_kind_appears() {
        let messages = generator(37).take(5_000);
        let kinds: HashSet<_> = messages
            .iter()
            .flat_map(|message| message.author().badges().iter().map(|badge| badge.kind()))
            .collect();

        assert!(kinds.contains(&BadgeKind::Broadcaster));
        assert!(kinds.contains(&BadgeKind::Moderator));
        assert!(kinds.contains(&BadgeKind::Vip));
        assert!(kinds.contains(&BadgeKind::Subscriber));
        assert!(kinds.contains(&BadgeKind::Prime));
        assert!(kinds.contains(&BadgeKind::Staff));
    }
}
