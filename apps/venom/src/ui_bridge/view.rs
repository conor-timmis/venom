//! Slint callback wiring and harness status publication.

use std::{
    cell::Cell,
    rc::{Rc, Weak},
};

use slint::{ComponentHandle, ModelRc, SharedString};

use crate::{MainWindow, config::MAX_BURST_MESSAGES, feed::ChatFeed, format::count_label};

/// Owns UI-only counters and translates Slint callbacks into feed actions.
pub struct ChatView {
    window: slint::Weak<MainWindow>,
    feed: Rc<ChatFeed>,
    rows_created: Cell<u64>,
}

impl ChatView {
    pub fn new(window: &MainWindow, feed: Rc<ChatFeed>) -> Rc<Self> {
        window.set_messages(ModelRc::from(feed.model()));
        window.set_retention_limit(to_i32(feed.capacity() as u64));

        let view = Rc::new(Self {
            window: window.as_weak(),
            feed,
            rows_created: Cell::new(0),
        });
        view.publish_stats();
        view
    }

    pub fn bind(self: &Rc<Self>, window: &MainWindow) {
        let weak = Rc::downgrade(self);

        window.on_set_live_rate({
            let weak = weak.clone();
            move |rate| with_view(&weak, |view| view.set_live_rate(rate))
        });
        window.on_add_messages({
            let weak = weak.clone();
            move |count| with_view(&weak, |view| view.add_messages(count))
        });
        window.on_clear_messages({
            let weak = weak.clone();
            move || with_view(&weak, ChatView::clear_messages)
        });
        window.on_load_history({
            let weak = weak.clone();
            move || with_view(&weak, ChatView::load_history)
        });
        window.on_tick({
            let weak = weak.clone();
            move || with_view(&weak, ChatView::tick)
        });
        window.on_row_created(move || with_view(&weak, ChatView::record_row_created));
    }

    /// Loads the configured workload after callbacks and the model are bound.
    pub fn start(&self) {
        self.feed.reload();
        self.rows_created.set(0);
        self.publish_stats();
    }

    fn set_live_rate(&self, rate: i32) {
        let Ok(rate) = u32::try_from(rate) else {
            tracing::warn!(rate, "ignoring negative live rate");
            return;
        };

        if !self.feed.set_rate(rate) {
            tracing::warn!(rate, "ignoring unsupported live rate");
            return;
        }

        tracing::info!(rate, "live message rate changed");
        self.publish_stats();
    }

    fn add_messages(&self, count: i32) {
        let Ok(count) = usize::try_from(count) else {
            tracing::warn!(count, "ignoring negative message batch");
            return;
        };

        let count = count.min(MAX_BURST_MESSAGES);
        if count == 0 {
            return;
        }

        self.feed.append_batch(count);
        self.publish_stats();
    }

    fn clear_messages(&self) {
        self.feed.clear();
        self.rows_created.set(0);
        self.publish_stats();
    }

    fn load_history(&self) {
        self.feed.reload();
        self.rows_created.set(0);
        self.publish_stats();
    }

    fn tick(&self) {
        self.feed.tick();
        self.publish_stats();
    }

    fn record_row_created(&self) {
        let count = self.rows_created.get().saturating_add(1);
        self.rows_created.set(count);

        if let Some(window) = self.window.upgrade() {
            window.set_rows_created(to_i32(count));
        }
    }

    fn publish_stats(&self) {
        let Some(window) = self.window.upgrade() else {
            return;
        };

        let snapshot = self.feed.snapshot();
        window.set_retained_count(to_i32(snapshot.retained as u64));
        window.set_generated_label(SharedString::from(count_label(snapshot.generated)));
        window.set_evicted_label(SharedString::from(count_label(snapshot.evicted)));
        window.set_last_insert_micros(to_i32(snapshot.last_insert_micros));
        window.set_worst_insert_micros(to_i32(snapshot.worst_insert_micros));
        window.set_asset_cache_label(SharedString::from(format!(
            "emotes {}/{}  ·  cache {}h/{}m",
            snapshot.asset_entries,
            snapshot.asset_capacity,
            snapshot.asset_hits,
            snapshot.asset_misses
        )));
        window.set_rows_created(to_i32(self.rows_created.get()));
        window.set_live_rate(to_i32(u64::from(snapshot.rate)));
        window.set_revision(to_i32(snapshot.generated));
    }
}

fn with_view(weak: &Weak<ChatView>, action: impl FnOnce(&ChatView)) {
    if let Some(view) = weak.upgrade() {
        action(&view);
    }
}

fn to_i32(value: u64) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}
