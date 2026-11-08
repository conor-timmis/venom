use crate::{MessageBadge, UserId};

/// A participant in chat.
///
/// Display names are not identities: equality is based on the platform-scoped
/// [`UserId`]. Two platforms may show the same name for different people.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChatUser {
    pub id: UserId,
    pub display_name: String,
    /// Badges shown before the display name. Empty until an adapter or the
    /// synthetic generator supplies them.
    pub badges: Vec<MessageBadge>,
}

impl ChatUser {
    pub fn new(id: UserId, display_name: impl Into<String>) -> Self {
        let display_name = display_name.into();
        let display_name = display_name.trim();
        debug_assert!(
            !display_name.is_empty(),
            "user display name must not be empty"
        );

        Self {
            id,
            display_name: display_name.to_owned(),
            badges: Vec::new(),
        }
    }

    /// Attaches badges while preserving their display order.
    #[must_use]
    pub fn with_badges(mut self, badges: Vec<MessageBadge>) -> Self {
        self.badges = badges;
        self
    }

    #[must_use]
    pub fn badges(&self) -> &[MessageBadge] {
        &self.badges
    }
}

#[cfg(test)]
mod tests {
    use super::ChatUser;
    use crate::{BadgeKind, MessageBadge, UserId};

    #[test]
    fn users_with_the_same_name_but_different_ids_are_distinct() {
        let twitch_user = ChatUser::new(UserId::new("twitch:42"), "pixel");
        let youtube_user = ChatUser::new(UserId::new("youtube:42"), "pixel");

        assert_eq!(twitch_user.display_name, youtube_user.display_name);
        assert_ne!(twitch_user, youtube_user);
    }

    #[test]
    fn new_users_start_without_badges() {
        let user = ChatUser::new(UserId::new("twitch:42"), "pixel");

        assert!(user.badges().is_empty());
    }

    #[test]
    fn badges_keep_the_order_they_were_attached_in() {
        let user = ChatUser::new(UserId::new("twitch:42"), "pixel").with_badges(vec![
            MessageBadge::new("twitch:moderator", "MOD", BadgeKind::Moderator),
            MessageBadge::new("twitch:subscriber", "SUB", BadgeKind::Subscriber),
        ]);

        let labels: Vec<_> = user.badges().iter().map(MessageBadge::label).collect();
        assert_eq!(labels, ["MOD", "SUB"]);
    }
}
