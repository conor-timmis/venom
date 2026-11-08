//! Bounded Slint model for the prototype message list.
//!
//! `VecModel` is convenient but removing from the front shifts every remaining
//! row and emits one notification per removal. This model keeps a `VecDeque`,
//! removes overflow in one batch, and appends the incoming batch with a single
//! row-added notification. Retention is therefore bounded by construction.

use std::{cell::RefCell, collections::VecDeque};

use slint::{Model, ModelNotify, ModelTracker};
use venom_domain::MessageId;

use crate::ChatRow;

/// One retained row plus the identity needed to prune reply lookups.
pub struct MessageEntry {
    id: MessageId,
    row: ChatRow,
}

impl MessageEntry {
    #[must_use]
    pub fn new(id: MessageId, row: ChatRow) -> Self {
        Self { id, row }
    }

    #[must_use]
    pub fn id(&self) -> &MessageId {
        &self.id
    }
}

/// Slint-facing row model with a fixed capacity.
pub struct MessageListModel {
    entries: RefCell<VecDeque<MessageEntry>>,
    capacity: usize,
    notify: ModelNotify,
}

impl MessageListModel {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "message list capacity must be positive");
        Self {
            entries: RefCell::new(VecDeque::with_capacity(capacity)),
            capacity,
            notify: ModelNotify::default(),
        }
    }

    /// Appends rows and returns everything evicted to make room.
    pub fn push_batch(&self, mut incoming: Vec<MessageEntry>) -> Vec<MessageEntry> {
        if incoming.is_empty() {
            return Vec::new();
        }

        let mut entries = self.entries.borrow_mut();

        if incoming.len() >= self.capacity {
            let mut evicted = std::mem::take(&mut *entries)
                .into_iter()
                .collect::<Vec<_>>();
            let keep_from = incoming.len() - self.capacity;
            let kept = incoming.split_off(keep_from);
            evicted.extend(incoming);
            *entries = kept.into_iter().collect();
            self.notify.reset();
            return evicted;
        }

        let overflow = entries
            .len()
            .saturating_add(incoming.len())
            .saturating_sub(self.capacity);
        let evicted = if overflow > 0 {
            let removed = entries.drain(..overflow).collect();
            self.notify.row_removed(0, overflow);
            removed
        } else {
            Vec::new()
        };

        let start = entries.len();
        let added = incoming.len();
        entries.extend(incoming);
        self.notify.row_added(start, added);

        evicted
    }

    pub fn clear(&self) {
        let mut entries = self.entries.borrow_mut();
        if entries.is_empty() {
            return;
        }

        entries.clear();
        self.notify.reset();
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.borrow().len()
    }

    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Model for MessageListModel {
    type Data = ChatRow;

    fn row_count(&self) -> usize {
        self.entries.borrow().len()
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        self.entries
            .borrow()
            .get(row)
            .map(|entry| entry.row.clone())
    }

    fn model_tracker(&self) -> &dyn ModelTracker {
        &self.notify
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use slint::{Color, Model, SharedString};

    use super::{MessageEntry, MessageListModel};
    use crate::ChatRow;
    use venom_domain::MessageId;

    fn row(index: usize) -> ChatRow {
        ChatRow {
            platform_label: SharedString::from("TWITCH"),
            platform_accent: Color::from_rgb_u8(0x9d, 0x6b, 0xff),
            author: SharedString::from("tester"),
            author_color: Color::from_rgb_u8(0xff, 0xff, 0xff),
            timestamp: SharedString::from("18:30:00"),
            body: SharedString::from(format!("message {index}")),
            reply_preview: SharedString::default(),
        }
    }

    fn entry(index: usize) -> MessageEntry {
        MessageEntry::new(MessageId::new(format!("message-{index}")), row(index))
    }

    #[test]
    fn rows_keep_insertion_order() {
        let model = MessageListModel::new(10);
        let evicted = model.push_batch(vec![entry(1), entry(2), entry(3)]);

        assert!(evicted.is_empty());
        assert_eq!(model.row_count(), 3);
        assert_eq!(model.row_data(0).expect("row 0").body.as_str(), "message 1");
        assert_eq!(model.row_data(2).expect("row 2").body.as_str(), "message 3");
        assert!(model.row_data(3).is_none());
    }

    #[test]
    fn overflow_evicts_the_oldest_rows_first() {
        let model = MessageListModel::new(3);
        model.push_batch(vec![entry(1), entry(2), entry(3)]);

        let evicted = model.push_batch(vec![entry(4), entry(5)]);

        assert_eq!(model.row_count(), 3);
        assert_eq!(
            evicted
                .iter()
                .map(|entry| entry.id().as_str())
                .collect::<Vec<_>>(),
            ["message-1", "message-2"]
        );
        assert_eq!(model.row_data(0).expect("row 0").body.as_str(), "message 3");
        assert_eq!(model.row_data(2).expect("row 2").body.as_str(), "message 5");
    }

    #[test]
    fn oversized_batch_keeps_only_the_newest_capacity() {
        let model = MessageListModel::new(3);
        model.push_batch(vec![entry(1)]);

        let evicted = model.push_batch(vec![entry(2), entry(3), entry(4), entry(5)]);

        assert_eq!(model.row_count(), 3);
        assert_eq!(
            evicted
                .iter()
                .map(|entry| entry.id().as_str())
                .collect::<Vec<_>>(),
            ["message-1", "message-2"]
        );
        assert_eq!(model.row_data(0).expect("row 0").body.as_str(), "message 3");
        assert_eq!(model.row_data(2).expect("row 2").body.as_str(), "message 5");
    }

    #[test]
    fn repeated_batches_never_exceed_capacity() {
        let model = MessageListModel::new(100);

        for batch in 0..20 {
            model.push_batch((0..25).map(|offset| entry(batch * 25 + offset)).collect());
            assert!(model.len() <= model.capacity());
        }
    }

    #[test]
    fn clear_removes_every_row() {
        let model = MessageListModel::new(10);
        model.push_batch(vec![entry(1), entry(2)]);

        model.clear();

        assert_eq!(model.row_count(), 0);
        assert!(model.row_data(0).is_none());
    }
}
