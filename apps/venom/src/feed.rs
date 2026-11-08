//! Synthetic feed orchestration for the prototype.
//!
//! The feed owns the generator, the bounded Slint model, reply preview lookup,
//! and harness counters. All mutation happens on the Slint thread; there is no
//! background producer in Sprint 2.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    time::Instant,
};

use venom_core::{SyntheticChatGenerator, SyntheticConfig, SyntheticConfigError, default_channels};
use venom_domain::MessageId;

use crate::{
    config::{ALLOWED_LIVE_RATES, HarnessConfig, LIVE_TICK_HZ},
    format::Clock,
    metrics::InsertMetrics,
    ui_bridge::{
        message_mapper::{self, ReplySummary},
        model::{MessageEntry, MessageListModel},
    },
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FeedSnapshot {
    pub retained: usize,
    pub generated: u64,
    pub evicted: u64,
    pub last_insert_micros: u64,
    pub worst_insert_micros: u64,
    pub rate: u32,
}

/// State shared by the UI bridge and the headless harness.
pub struct ChatFeed {
    generator: RefCell<SyntheticChatGenerator>,
    model: Rc<MessageListModel>,
    replies: RefCell<HashMap<MessageId, ReplySummary>>,
    clock: Clock,
    initial_messages: usize,
    rate: Cell<u32>,
    rate_remainder: Cell<u32>,
    metrics: RefCell<InsertMetrics>,
    total_generated: Cell<u64>,
    total_evicted: Cell<u64>,
}

impl ChatFeed {
    pub fn new(config: &HarnessConfig, clock: Clock) -> Result<Self, SyntheticConfigError> {
        let synthetic = SyntheticConfig::new(config.seed, default_channels())?;

        Ok(Self {
            generator: RefCell::new(SyntheticChatGenerator::new(synthetic)),
            model: Rc::new(MessageListModel::new(config.retained_messages)),
            replies: RefCell::new(HashMap::new()),
            clock,
            initial_messages: config.initial_messages,
            rate: Cell::new(config.live_rate),
            rate_remainder: Cell::new(0),
            metrics: RefCell::new(InsertMetrics::default()),
            total_generated: Cell::new(0),
            total_evicted: Cell::new(0),
        })
    }

    #[must_use]
    pub fn model(&self) -> Rc<MessageListModel> {
        self.model.clone()
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.model.capacity()
    }

    #[must_use]
    pub fn snapshot(&self) -> FeedSnapshot {
        let metrics = *self.metrics.borrow();
        FeedSnapshot {
            retained: self.model.len(),
            generated: self.total_generated.get(),
            evicted: self.total_evicted.get(),
            last_insert_micros: metrics.last_micros(),
            worst_insert_micros: metrics.worst_micros(),
            rate: self.rate.get(),
        }
    }

    /// Reloads the configured history and restarts the synthetic stream.
    pub fn reload(&self) {
        self.reset();
        self.append_batch(self.initial_messages);
    }

    /// Clears visible state and restarts the synthetic stream at its origin.
    pub fn clear(&self) {
        self.reset();
    }

    /// Returns false when `rate` is not one of the harness options.
    pub fn set_rate(&self, rate: u32) -> bool {
        if !ALLOWED_LIVE_RATES.contains(&rate) {
            return false;
        }

        self.rate.set(rate);
        self.rate_remainder.set(0);
        true
    }

    /// Advances the feed by one UI tick, preserving fractional messages.
    pub fn tick(&self) {
        let rate = self.rate.get();
        if rate == 0 {
            return;
        }

        let accumulated = self.rate_remainder.get().saturating_add(rate);
        self.rate_remainder.set(accumulated % LIVE_TICK_HZ);

        let count = accumulated / LIVE_TICK_HZ;
        if count > 0 {
            self.append_batch(count as usize);
        }
    }

    /// Generates and inserts up to one full window of messages.
    ///
    /// Callers that expose a manual burst control apply their own limit in the
    /// UI bridge; prefill must be able to fill the entire retention window.
    pub fn append_batch(&self, count: usize) {
        let count = count.min(self.capacity());
        if count == 0 {
            return;
        }

        let started = Instant::now();
        let generated = self.generator.borrow_mut().take(count);
        let mut entries = Vec::with_capacity(generated.len());

        {
            let mut replies = self.replies.borrow_mut();
            for message in &generated {
                let reply = message
                    .reply_to()
                    .and_then(|message_id| replies.get(message_id))
                    .cloned();
                let row = message_mapper::to_chat_row(message, reply.as_ref(), self.clock);
                entries.push(MessageEntry::new(message.id().clone(), row));
                replies.insert(message.id().clone(), ReplySummary::from_message(message));
            }
        }

        let evicted = self.model.push_batch(entries);

        {
            let mut replies = self.replies.borrow_mut();
            for entry in &evicted {
                replies.remove(entry.id());
            }
        }

        self.metrics.borrow_mut().record(started.elapsed());
        self.total_generated
            .set(self.total_generated.get().saturating_add(count as u64));
        self.total_evicted.set(
            self.total_evicted
                .get()
                .saturating_add(evicted.len() as u64),
        );
    }

    /// Test hook: how many reply targets are currently indexed.
    #[cfg(test)]
    pub fn reply_targets(&self) -> usize {
        self.replies.borrow().len()
    }

    fn reset(&self) {
        self.model.clear();
        self.replies.borrow_mut().clear();
        self.generator.borrow_mut().reset();
        self.metrics.borrow_mut().reset();
        self.rate_remainder.set(0);
        self.total_generated.set(0);
        self.total_evicted.set(0);
    }
}

#[cfg(test)]
mod tests {
    use slint::Model;
    use time::UtcOffset;

    use super::ChatFeed;
    use crate::{
        config::{HarnessConfig, MAX_BURST_MESSAGES},
        format::Clock,
    };
    use venom_core::DEFAULT_SEED;

    fn config(initial_messages: usize, retained_messages: usize, live_rate: u32) -> HarnessConfig {
        HarnessConfig {
            headless: true,
            initial_messages,
            retained_messages,
            seed: DEFAULT_SEED,
            live_rate,
            exit_after: None,
        }
    }

    fn feed(initial_messages: usize, retained_messages: usize, live_rate: u32) -> ChatFeed {
        ChatFeed::new(
            &config(initial_messages, retained_messages, live_rate),
            Clock::fixed(UtcOffset::UTC),
        )
        .expect("default channels are present")
    }

    #[test]
    fn reload_loads_the_configured_history() {
        let feed = feed(25, 100, 0);
        feed.reload();

        let snapshot = feed.snapshot();
        assert_eq!(snapshot.retained, 25);
        assert_eq!(snapshot.generated, 25);
        assert_eq!(snapshot.evicted, 0);
    }

    #[test]
    fn reload_loads_a_history_larger_than_the_manual_burst_cap() {
        let history = MAX_BURST_MESSAGES + 1;
        let feed = feed(history, history, 0);

        feed.reload();

        let snapshot = feed.snapshot();
        assert_eq!(snapshot.retained, history);
        assert_eq!(snapshot.generated, history as u64);
        assert_eq!(snapshot.evicted, 0);
    }

    #[test]
    fn tick_emits_whole_messages_at_the_configured_rate() {
        let feed = feed(0, 100, 60);

        feed.tick();

        assert_eq!(feed.snapshot().generated, 3);
        assert_eq!(feed.snapshot().retained, 3);
    }

    #[test]
    fn fractional_rates_carry_the_remainder_between_ticks() {
        let feed = feed(0, 100, 30);

        for _ in 0..4 {
            feed.tick();
        }

        assert_eq!(feed.snapshot().generated, 6);
    }

    #[test]
    fn retention_evicts_old_rows_and_stays_bounded() {
        let feed = feed(0, 10, 60);

        for _ in 0..5 {
            feed.tick();
        }

        let snapshot = feed.snapshot();
        assert_eq!(snapshot.retained, 10);
        assert_eq!(snapshot.generated, 15);
        assert_eq!(snapshot.evicted, 5);
        assert_eq!(feed.model().row_count(), 10);
    }

    #[test]
    fn reply_previews_resolve_while_the_target_is_retained() {
        // The generator draws reply targets from the last 128 messages per
        // channel, so a retention window that comfortably exceeds that
        // history keeps every target resident and every reply resolvable.
        let feed = feed(0, 512, 0);

        feed.append_batch(512);

        let model = feed.model();
        let previewed = (0..model.row_count())
            .filter(|index| {
                model
                    .row_data(*index)
                    .is_some_and(|row| !row.reply_preview.is_empty())
            })
            .count();

        assert!(previewed > 0, "retained replies should render previews");
    }

    #[test]
    fn evicted_messages_release_their_reply_lookups() {
        let feed = feed(0, 32, 0);

        feed.append_batch(32);
        assert_eq!(feed.reply_targets(), 32);

        feed.append_batch(32);
        assert_eq!(
            feed.reply_targets(),
            32,
            "the reply index must stay bounded by retention"
        );
    }

    #[test]
    fn reload_and_clear_reset_harness_counters() {
        let feed = feed(5, 10, 60);
        feed.reload();
        feed.tick();
        feed.reload();

        let snapshot = feed.snapshot();
        assert_eq!(snapshot.retained, 5);
        assert_eq!(snapshot.generated, 5);
        assert_eq!(snapshot.evicted, 0);

        feed.clear();
        let snapshot = feed.snapshot();
        assert_eq!(snapshot.retained, 0);
        assert_eq!(snapshot.generated, 0);
    }

    #[test]
    fn unsupported_rates_are_ignored() {
        let feed = feed(0, 10, 60);

        assert!(!feed.set_rate(45));
        assert!(feed.set_rate(120));
        assert_eq!(feed.snapshot().rate, 120);
    }

    #[test]
    fn manual_batches_are_capped_by_retention() {
        let feed = feed(0, 10, 0);

        feed.append_batch(MAX_BURST_MESSAGES * 2);

        assert_eq!(feed.snapshot().retained, 10);
        assert_eq!(feed.snapshot().generated, 10);
    }
}
