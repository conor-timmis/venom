/// The category a chat badge belongs to.
///
/// The renderer maps kinds to colours and widths; adapters map platform badge
/// sets onto these shared categories. `Other` keeps unknown platform badges
/// displayable without leaking platform-specific enums into the domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BadgeKind {
    Broadcaster,
    Moderator,
    Vip,
    Subscriber,
    Prime,
    Staff,
    Other,
}

impl BadgeKind {
    /// Stable lowercase identifier used in diagnostics.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Broadcaster => "broadcaster",
            Self::Moderator => "moderator",
            Self::Vip => "vip",
            Self::Subscriber => "subscriber",
            Self::Prime => "prime",
            Self::Staff => "staff",
            Self::Other => "other",
        }
    }
}

/// A short mark shown inline before a participant's display name.
///
/// Badges are user metadata rather than message content, matching how chat
/// platforms attach them to a participant (MAIN.md §6.1, §10.5). The platform
/// owns the identifier, so callers should scope it (for example
/// `twitch:moderator`) just like [`UserId`](crate::UserId).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MessageBadge {
    id: String,
    label: String,
    kind: BadgeKind,
}

impl MessageBadge {
    pub fn new(id: impl Into<String>, label: impl Into<String>, kind: BadgeKind) -> Self {
        let id = id.into();
        let label = label.into();
        let id = id.trim();
        let label = label.trim();
        debug_assert!(!id.is_empty(), "badge id must not be empty");
        debug_assert!(!label.is_empty(), "badge label must not be empty");

        Self {
            id: id.to_owned(),
            label: label.to_owned(),
            kind,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Text shown inside the badge when assets are unavailable.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    #[must_use]
    pub const fn kind(&self) -> BadgeKind {
        self.kind
    }
}

#[cfg(test)]
mod tests {
    use super::{BadgeKind, MessageBadge};

    #[test]
    fn badge_metadata_is_trimmed_and_preserved() {
        let badge = MessageBadge::new(" twitch:moderator ", " MOD ", BadgeKind::Moderator);

        assert_eq!(badge.id(), "twitch:moderator");
        assert_eq!(badge.label(), "MOD");
        assert_eq!(badge.kind(), BadgeKind::Moderator);
    }

    #[test]
    fn badge_kinds_have_stable_identifiers() {
        assert_eq!(BadgeKind::Broadcaster.as_str(), "broadcaster");
        assert_eq!(BadgeKind::Subscriber.as_str(), "subscriber");
        assert_eq!(BadgeKind::Other.as_str(), "other");
    }

    #[test]
    fn badges_compare_by_all_fields() {
        let moderator = MessageBadge::new("twitch:moderator", "MOD", BadgeKind::Moderator);
        let same = MessageBadge::new("twitch:moderator", "MOD", BadgeKind::Moderator);
        let other_label = MessageBadge::new("twitch:moderator", "MODS", BadgeKind::Moderator);

        assert_eq!(moderator, same);
        assert_ne!(moderator, other_label);
    }
}
