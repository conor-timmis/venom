//! Message body composition.
//!
//! The Phase 0 renderer flattens segments to plain text, but the generator
//! still produces structured segments so Sprint 3 can render emotes, links, and
//! mentions without changing the message source.

use fastrand::Rng;
use venom_domain::{ChatUser, MessageSegment};

use super::{
    names,
    random::{chance, pick, pick_index},
};

const SHORT_WEIGHT: u32 = 30;
const MEDIUM_WEIGHT: u32 = 50;
const LONG_WEIGHT: u32 = 20;

const EMOTE_PERCENT: u32 = 26;
const LINK_PERCENT: u32 = 8;
const MENTION_PERCENT: u32 = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MessageProfile {
    Short,
    Medium,
    Long,
}

impl MessageProfile {
    const ALL: [Self; 3] = [Self::Short, Self::Medium, Self::Long];

    const fn weight(self) -> u32 {
        match self {
            Self::Short => SHORT_WEIGHT,
            Self::Medium => MEDIUM_WEIGHT,
            Self::Long => LONG_WEIGHT,
        }
    }

    const fn pool(self) -> &'static [&'static str] {
        match self {
            Self::Short => names::SHORT_MESSAGES,
            Self::Medium => names::MEDIUM_MESSAGES,
            Self::Long => names::LONG_MESSAGES,
        }
    }
}

/// Builds one message body.
///
/// The first segment is always text, so every generated message has a
/// meaningful plain-text projection even before rich rendering exists.
pub(super) fn compose(
    rng: &mut Rng,
    author: &ChatUser,
    authors: &[ChatUser],
) -> Vec<MessageSegment> {
    let mut segments = Vec::with_capacity(4);
    segments.push(MessageSegment::Text(pick_body(rng).to_owned()));

    if chance(rng, EMOTE_PERCENT) {
        append_emote(rng, &mut segments);
    }

    if chance(rng, LINK_PERCENT) {
        append_link(rng, &mut segments);
    }

    if chance(rng, MENTION_PERCENT) {
        append_mention(rng, author, authors, &mut segments);
    }

    segments
}

fn pick_body(rng: &mut Rng) -> &'static str {
    let total_weight: u32 = MessageProfile::ALL
        .iter()
        .map(|profile| profile.weight())
        .sum();
    let mut roll = rng.u32(..total_weight);

    for profile in MessageProfile::ALL {
        let weight = profile.weight();
        if roll < weight {
            return pick(rng, profile.pool());
        }
        roll -= weight;
    }

    debug_assert!(false, "message profile weights must cover every roll");
    names::SHORT_MESSAGES[0]
}

fn append_emote(rng: &mut Rng, segments: &mut Vec<MessageSegment>) {
    let emote = pick(rng, names::EMOTES);
    segments.push(MessageSegment::Text(" ".to_owned()));
    segments.push(MessageSegment::Emote {
        id: format!("{}:{}", emote.provider.as_str(), emote.name),
        name: emote.name.to_owned(),
        provider: emote.provider,
    });
}

fn append_link(rng: &mut Rng, segments: &mut Vec<MessageSegment>) {
    let link = pick(rng, names::LINKS);
    segments.push(MessageSegment::Text(" ".to_owned()));
    segments.push(MessageSegment::Link {
        label: link.label.to_owned(),
        url: link.url.to_owned(),
    });
}

fn append_mention(
    rng: &mut Rng,
    author: &ChatUser,
    authors: &[ChatUser],
    segments: &mut Vec<MessageSegment>,
) {
    if authors.len() < 2 {
        return;
    }

    let mut index = pick_index(rng, authors.len());
    if authors[index].id == author.id {
        index = (index + 1) % authors.len();
    }

    let mentioned = &authors[index];
    segments.push(MessageSegment::Text(" ".to_owned()));
    segments.push(MessageSegment::Mention {
        user_id: mentioned.id.as_str().to_owned(),
        display_name: mentioned.display_name.clone(),
    });
}

#[cfg(test)]
mod tests {
    use fastrand::Rng;
    use venom_domain::{ChatUser, MessageSegment, UserId};

    use super::compose;

    fn authors() -> Vec<ChatUser> {
        vec![
            ChatUser::new(UserId::new("twitch:user:1"), "moonlit_mara"),
            ChatUser::new(UserId::new("twitch:user:2"), "pixelpilot"),
            ChatUser::new(UserId::new("twitch:user:3"), "GG_Bandit"),
        ]
    }

    #[test]
    fn every_message_starts_with_text_and_has_plain_content() {
        let mut rng = Rng::with_seed(7);
        let authors = authors();

        for _ in 0..500 {
            let segments = compose(&mut rng, &authors[0], &authors);
            assert!(matches!(segments.first(), Some(MessageSegment::Text(_))));

            let plain = segments
                .iter()
                .map(|segment| {
                    let mut text = String::new();
                    segment.write_plain_text(&mut text);
                    text
                })
                .collect::<String>();
            assert!(!plain.trim().is_empty());
        }
    }

    #[test]
    fn mentions_never_target_the_author() {
        let mut rng = Rng::with_seed(11);
        let authors = authors();
        let author = &authors[0];

        for _ in 0..2_000 {
            let segments = compose(&mut rng, author, &authors);
            for segment in segments {
                if let MessageSegment::Mention { user_id, .. } = segment {
                    assert_ne!(user_id, author.id.as_str());
                }
            }
        }
    }

    #[test]
    fn compose_is_reproducible_for_a_seed() {
        let mut first = Rng::with_seed(99);
        let mut second = Rng::with_seed(99);
        let authors = authors();

        for _ in 0..100 {
            assert_eq!(
                compose(&mut first, &authors[1], &authors),
                compose(&mut second, &authors[1], &authors)
            );
        }
    }

    #[test]
    fn rich_segments_eventually_appear() {
        let mut rng = Rng::with_seed(23);
        let authors = authors();
        let mut saw_emote = false;
        let mut saw_link = false;
        let mut saw_mention = false;

        for _ in 0..2_000 {
            for segment in compose(&mut rng, &authors[0], &authors) {
                match segment {
                    MessageSegment::Emote { .. } => saw_emote = true,
                    MessageSegment::Link { .. } => saw_link = true,
                    MessageSegment::Mention { .. } => saw_mention = true,
                    MessageSegment::Text(_) => {}
                }
            }
        }

        assert!(saw_emote);
        assert!(saw_link);
        assert!(saw_mention);
    }
}
