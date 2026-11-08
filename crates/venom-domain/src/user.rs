use crate::UserId;

/// A participant in chat.
///
/// Display names are not identities: equality is based on the platform-scoped
/// [`UserId`]. Two platforms may show the same name for different people.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChatUser {
    pub id: UserId,
    pub display_name: String,
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ChatUser;
    use crate::UserId;

    #[test]
    fn users_with_the_same_name_but_different_ids_are_distinct() {
        let twitch_user = ChatUser::new(UserId::new("twitch:42"), "pixel");
        let youtube_user = ChatUser::new(UserId::new("youtube:42"), "pixel");

        assert_eq!(twitch_user.display_name, youtube_user.display_name);
        assert_ne!(twitch_user, youtube_user);
    }
}
