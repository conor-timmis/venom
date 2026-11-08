//! Read-only pools used to compose synthetic chat.
//!
//! Keeping content in data tables makes the generator itself about state and
//! ordering. Pools are deliberately small but varied enough to exercise wrapped
//! rows, short one-liners, emotes, links, and mentions.

use venom_domain::{BadgeKind, EmoteProvider};

pub(super) struct EmoteSpec {
    pub name: &'static str,
    pub provider: EmoteProvider,
}

pub(super) struct BadgeSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: BadgeKind,
}

pub(super) struct LinkSpec {
    pub label: &'static str,
    pub url: &'static str,
}

pub(super) const USER_NAMES: &[&str] = &[
    "moonlit_mara",
    "pixelpilot",
    "GG_Bandit",
    "NovaDrift",
    "quietstorm",
    "toastcrumb",
    "hexadecimal",
    "LazyOtter",
    "vaporwave_",
    "RinTheFox",
    "caffeinecow",
    "sable_shift",
    "TheRealKJ",
    "bitcrushbunny",
    "OrbitalOwl",
    "snackattack",
    "lucid_dream",
    "frostbyte",
    "MangoSorbet",
    "nightjar",
    "PlasmaPanda",
    "ZeroCool",
    "tangerine",
    "kaiju_kid",
    "softreset",
    "PixelPope",
    "WanderingWren",
    "CircuitSamurai",
];

pub(super) const SHORT_MESSAGES: &[&str] = &[
    "hey chat",
    "GG",
    "that was clean",
    "no way",
    "one more game",
    "chat is cooking today",
    "clip it",
    "we go again",
    "good morning",
    "what a save",
    "LETS GO",
    "o7",
    "who clipped that",
    "that timing though",
    "first time here, loving the stream",
];

pub(super) const MEDIUM_MESSAGES: &[&str] = &[
    "the new movement feels so much better than last patch",
    "honestly that boss fight is way harder than people give it credit for",
    "is the stream audio a little quiet for anyone else or is it just me",
    "i have been waiting for this collab for months and it is finally happening",
    "the chat lag is real today but the vibes are immaculate",
    "that was the most unnecessary jump scare i have ever seen",
    "can we appreciate how good the new menu theme is",
    "not me missing the only important part of the run",
    "the mods are doing overtime tonight and we appreciate them",
    "what settings are you using for the lighting? it looks incredible",
    "i keep forgetting this game has a photo mode and then chat posts this",
    "the way the chat instantly spams the same emote is beautiful chaos",
    "unpopular opinion: the slower pacing actually makes the story better",
    "someone please tell me the name of that song in the intro",
    "this is the most relaxed high-stakes run i have ever watched",
    "the community goal is going to be hit before the next break",
    "that inventory management was more stressful than the actual fight",
    "the patch notes said minor changes and then they changed everything",
];

pub(super) const LONG_MESSAGES: &[&str] = &[
    "okay so here is my completely unsolicited take on the new patch: the movement feels great, the netcode is smoother, but the ranked placement matches are still a coin flip. i think they need another week before the season starts.",
    "the thing that gets me is how the lighting changes when you walk into the cave. the whole colour palette shifts and suddenly the UI feels like a different game. i kinda love it, even if my fps drops a little when the fog rolls in.",
    "chat i need help: my stream audio is fine in the recording but my headphones keep cutting out every ten minutes. is that an obs thing, a driver thing, or just my usb hub being haunted? i have already tried the obvious cable swap.",
    "for anyone new here, the run starts around twenty minutes in and the route changes depending on which upgrade drops from the first boss. the chat usually votes on the risky path, which is fun until it goes horribly wrong at the end.",
    "i genuinely think the side quests are the best part of this game. the main story is fine, but the little conversations in the town change depending on who you bring, and some of them completely recontextualise the ending.",
    "the way the community has turned a random background npc into the main character is exactly why i love watching this game. someone made fan art, someone else wrote a lore document, and now chat refuses to let it go.",
    "my hot take is that the difficulty options should be available from the start. the default experience is great, but not everyone has four hours to learn a boss pattern, and the accessibility settings are already doing most of the work anyway.",
    "that was an incredible run. the route looked impossible after the second split, the resource management was tight the whole way through, and the final dodge was pure luck. i am going to be thinking about this one for a while.",
];

pub(super) const EMOTES: &[EmoteSpec] = &[
    EmoteSpec {
        name: "Kappa",
        provider: EmoteProvider::Twitch,
    },
    EmoteSpec {
        name: "PogChamp",
        provider: EmoteProvider::Twitch,
    },
    EmoteSpec {
        name: "KEKW",
        provider: EmoteProvider::SevenTv,
    },
    EmoteSpec {
        name: "catJAM",
        provider: EmoteProvider::BetterTv,
    },
    EmoteSpec {
        name: "5Head",
        provider: EmoteProvider::SevenTv,
    },
    EmoteSpec {
        name: "monkaS",
        provider: EmoteProvider::FrankerFaceZ,
    },
    EmoteSpec {
        name: "Clap",
        provider: EmoteProvider::Twitch,
    },
    EmoteSpec {
        name: "Kreygasm",
        provider: EmoteProvider::Twitch,
    },
    EmoteSpec {
        name: "LULW",
        provider: EmoteProvider::SevenTv,
    },
    EmoteSpec {
        name: "PogU",
        provider: EmoteProvider::SevenTv,
    },
    EmoteSpec {
        name: "Pog",
        provider: EmoteProvider::YouTube,
    },
    EmoteSpec {
        name: "Hype",
        provider: EmoteProvider::Kick,
    },
    EmoteSpec {
        name: "PartyParrot",
        provider: EmoteProvider::BetterTv,
    },
    EmoteSpec {
        name: "DanceTime",
        provider: EmoteProvider::SevenTv,
    },
];

/// Badge catalogue shared by every synthetic platform.
///
/// Only the identifier is platform-scoped; the label and category are shared
/// so the renderer sees the same badge contract from every channel.
pub(super) const BADGES: &[BadgeSpec] = &[
    BadgeSpec {
        id: "broadcaster",
        label: "HOST",
        kind: BadgeKind::Broadcaster,
    },
    BadgeSpec {
        id: "moderator",
        label: "MOD",
        kind: BadgeKind::Moderator,
    },
    BadgeSpec {
        id: "vip",
        label: "VIP",
        kind: BadgeKind::Vip,
    },
    BadgeSpec {
        id: "subscriber",
        label: "SUB",
        kind: BadgeKind::Subscriber,
    },
    BadgeSpec {
        id: "prime",
        label: "PRIME",
        kind: BadgeKind::Prime,
    },
    BadgeSpec {
        id: "staff",
        label: "STAFF",
        kind: BadgeKind::Staff,
    },
];

pub(super) const LINKS: &[LinkSpec] = &[
    LinkSpec {
        label: "clip",
        url: "https://clips.twitch.tv/VenomPrototype",
    },
    LinkSpec {
        label: "patch notes",
        url: "https://venom.cx/notes",
    },
    LinkSpec {
        label: "the route",
        url: "https://venom.cx/route",
    },
    LinkSpec {
        label: "this moment",
        url: "https://youtube.com/watch?v=venom-prototype",
    },
];

#[cfg(test)]
mod tests {
    use super::{
        BADGES, EMOTES, LINKS, LONG_MESSAGES, MEDIUM_MESSAGES, SHORT_MESSAGES, USER_NAMES,
    };

    #[test]
    fn content_pools_are_not_empty() {
        assert!(!USER_NAMES.is_empty());
        assert!(!SHORT_MESSAGES.is_empty());
        assert!(!MEDIUM_MESSAGES.is_empty());
        assert!(!LONG_MESSAGES.is_empty());
        assert!(!EMOTES.is_empty());
        assert!(!LINKS.is_empty());
        assert!(!BADGES.is_empty());
    }

    #[test]
    fn every_message_has_visible_text() {
        for message in SHORT_MESSAGES
            .iter()
            .chain(MEDIUM_MESSAGES)
            .chain(LONG_MESSAGES)
        {
            assert!(!message.trim().is_empty());
        }
    }

    #[test]
    fn emote_and_link_metadata_is_complete() {
        assert!(EMOTES.iter().all(|emote| !emote.name.is_empty()));
        assert!(
            LINKS
                .iter()
                .all(|link| !link.label.is_empty() && link.url.starts_with("https://"))
        );
    }

    #[test]
    fn badge_metadata_is_complete() {
        assert!(
            BADGES
                .iter()
                .all(|badge| !badge.id.is_empty() && !badge.label.is_empty())
        );
    }
}
