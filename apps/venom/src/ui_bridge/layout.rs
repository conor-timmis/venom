//! Line breaking for rich message bodies.
//!
//! Slint has no flow layout that mixes text and images inline, so the Phase 0
//! prototype measures and wraps runs in Rust and renders each resulting line as
//! a `HorizontalLayout`. The measurement is approximate: character advances are
//! modelled from the 14 px body font instead of querying the text shaper.

use venom_domain::{EmoteProvider, MessageSegment};

/// Approximate advance of one body-font character.
pub const CHAR_WIDTH_PX: f32 = 7.0;
/// Approximate advance of a space in the body font.
pub const SPACE_WIDTH_PX: f32 = 4.0;
/// Rendered emote width.
pub const EMOTE_WIDTH_PX: f32 = 20.0;
/// Layout spacing between adjacent rich runs.
pub const TOKEN_GAP_PX: f32 = 4.0;
/// Conservative body width for the 1180 px prototype window.
pub const DEFAULT_BODY_WIDTH_PX: f32 = 1_060.0;

/// One run inside a wrapped line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    Link {
        label: String,
        url: String,
    },
    Mention {
        user_id: String,
        display_name: String,
    },
    Emote {
        id: String,
        name: String,
        provider: EmoteProvider,
    },
}

/// A single rendered line of inline runs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Line {
    segments: Vec<Inline>,
}

impl Line {
    #[must_use]
    pub fn segments(&self) -> &[Inline] {
        &self.segments
    }

    fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    fn push_text(&mut self, text: &str) {
        if let Some(Inline::Text(existing)) = self.segments.last_mut() {
            existing.push_str(text);
        } else {
            self.segments.push(Inline::Text(text.to_owned()));
        }
    }

    fn push_inline(&mut self, inline: Inline) {
        self.segments.push(inline);
    }
}

/// Wraps structured content into lines that fit `max_width_px`.
///
/// Whitespace is collapsed to single spaces, matching how the prototype body
/// currently renders. A single word wider than the line is kept whole rather
/// than broken mid-word.
#[must_use]
pub fn wrap(segments: &[MessageSegment], max_width_px: f32) -> Vec<Line> {
    let mut layout = Layout::new(max_width_px);

    for segment in segments {
        match segment {
            MessageSegment::Text(text) => layout.push_text(text),
            MessageSegment::Emote { id, name, provider } => layout.push_inline(
                Inline::Emote {
                    id: id.clone(),
                    name: name.clone(),
                    provider: *provider,
                },
                EMOTE_WIDTH_PX + TOKEN_GAP_PX,
            ),
            MessageSegment::Link { label, url } => layout.push_inline(
                Inline::Link {
                    label: label.clone(),
                    url: url.clone(),
                },
                text_width(label) + TOKEN_GAP_PX,
            ),
            MessageSegment::Mention {
                user_id,
                display_name,
            } => layout.push_inline(
                Inline::Mention {
                    user_id: user_id.clone(),
                    display_name: display_name.clone(),
                },
                text_width(&mention_text(display_name)) + TOKEN_GAP_PX,
            ),
        }
    }

    layout.finish()
}

/// Text a mention renders as, shared by measurement and rendering.
#[must_use]
pub fn mention_text(display_name: &str) -> String {
    format!("@{display_name}")
}

struct Layout {
    lines: Vec<Line>,
    current_width: f32,
    max_width: f32,
    pending_space: bool,
}

impl Layout {
    fn new(max_width_px: f32) -> Self {
        Self {
            lines: vec![Line::default()],
            current_width: 0.0,
            max_width: max_width_px.max(CHAR_WIDTH_PX),
            pending_space: false,
        }
    }

    fn push_text(&mut self, text: &str) {
        let mut word = String::new();

        for character in text.chars() {
            if character.is_whitespace() {
                self.flush_word(&mut word);
                self.pending_space = true;
            } else {
                word.push(character);
            }
        }

        self.flush_word(&mut word);
    }

    fn flush_word(&mut self, word: &mut String) {
        if word.is_empty() {
            return;
        }

        let word = std::mem::take(word);
        let width = text_width(&word);
        self.push_run(width, |line| line.push_text(&word));
    }

    fn push_inline(&mut self, inline: Inline, width: f32) {
        self.push_run(width, |line| line.push_inline(inline));
    }

    fn push_run(&mut self, width: f32, append: impl FnOnce(&mut Line)) {
        let line_is_empty = self.lines.last().is_some_and(Line::is_empty);
        let separator = if self.pending_space && !line_is_empty {
            SPACE_WIDTH_PX
        } else {
            0.0
        };

        if !line_is_empty && self.current_width + separator + width > self.max_width {
            self.begin_line();
        }

        if self.pending_space && !line_is_empty {
            self.current_width += SPACE_WIDTH_PX;
            self.line().push_text(" ");
        }

        self.pending_space = false;
        self.current_width += width;
        append(self.line());
    }

    fn begin_line(&mut self) {
        self.lines.push(Line::default());
        self.current_width = 0.0;
        self.pending_space = false;
    }

    fn line(&mut self) -> &mut Line {
        self.lines
            .last_mut()
            .expect("a layout always has an open line")
    }

    fn finish(self) -> Vec<Line> {
        self.lines
    }
}

fn text_width(text: &str) -> f32 {
    text.chars().count() as f32 * CHAR_WIDTH_PX
}

#[cfg(test)]
mod tests {
    use venom_domain::{EmoteProvider, MessageSegment};

    use super::{DEFAULT_BODY_WIDTH_PX, Inline, Line, mention_text, wrap};

    fn text(value: &str) -> MessageSegment {
        MessageSegment::Text(value.to_owned())
    }

    fn emote(name: &str) -> MessageSegment {
        MessageSegment::Emote {
            id: format!("twitch:{name}"),
            name: name.to_owned(),
            provider: EmoteProvider::Twitch,
        }
    }

    fn line_text(line: &Line) -> String {
        line.segments()
            .iter()
            .map(|segment| match segment {
                Inline::Text(text) => text.clone(),
                Inline::Link { label, .. } => label.clone(),
                Inline::Mention { display_name, .. } => mention_text(display_name),
                Inline::Emote { name, .. } => name.clone(),
            })
            .collect()
    }

    fn all_text(lines: &[Line]) -> String {
        lines.iter().map(line_text).collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn short_message_stays_on_one_line() {
        let lines = wrap(&[text("hello chat")], DEFAULT_BODY_WIDTH_PX);

        assert_eq!(lines.len(), 1);
        assert_eq!(line_text(&lines[0]), "hello chat");
    }

    #[test]
    fn long_message_wraps_into_multiple_lines() {
        let body = "word ".repeat(400);
        let lines = wrap(&[text(&body)], 200.0);

        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| !line.is_empty()));
    }

    #[test]
    fn every_word_survives_wrapping() {
        let body = "alpha beta gamma delta epsilon zeta eta theta iota kappa";
        let lines = wrap(&[text(body)], 100.0);

        assert_eq!(all_text(&lines), body);
    }

    #[test]
    fn emotes_are_atomic_tokens() {
        let segments = [text("hello "), emote("Kappa"), text(" chat")];
        let lines = wrap(&segments, DEFAULT_BODY_WIDTH_PX);

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].segments().len(), 3);
        assert!(matches!(
            lines[0].segments()[1],
            Inline::Emote { ref name, .. } if name == "Kappa"
        ));
        assert_eq!(line_text(&lines[0]), "hello Kappa chat");
    }

    #[test]
    fn links_and_mentions_are_atomic_tokens() {
        let segments = [
            text("see "),
            MessageSegment::Link {
                label: "the clip".to_owned(),
                url: "https://example.test/clip".to_owned(),
            },
            text(" and "),
            MessageSegment::Mention {
                user_id: "twitch:user:2".to_owned(),
                display_name: "moth".to_owned(),
            },
        ];

        let lines = wrap(&segments, DEFAULT_BODY_WIDTH_PX);

        assert_eq!(lines.len(), 1);
        assert!(matches!(lines[0].segments()[1], Inline::Link { .. }));
        assert!(matches!(lines[0].segments()[3], Inline::Mention { .. }));
        assert_eq!(line_text(&lines[0]), "see the clip and @moth");
    }

    #[test]
    fn leading_and_repeated_whitespace_collapses() {
        let lines = wrap(&[text("   hello    chat   ")], DEFAULT_BODY_WIDTH_PX);

        assert_eq!(line_text(&lines[0]), "hello chat");
    }

    #[test]
    fn segments_without_separating_space_stay_adjacent() {
        let lines = wrap(&[text("before"), emote("Kappa")], DEFAULT_BODY_WIDTH_PX);

        assert_eq!(line_text(&lines[0]), "beforeKappa");
    }

    #[test]
    fn zero_width_still_makes_progress() {
        let lines = wrap(&[text("alpha beta gamma")], 0.0);

        assert_eq!(lines.len(), 3);
        assert_eq!(all_text(&lines), "alpha beta gamma");
    }

    #[test]
    fn empty_input_yields_one_empty_line() {
        let lines = wrap(&[], DEFAULT_BODY_WIDTH_PX);

        assert_eq!(lines.len(), 1);
        assert!(lines[0].is_empty());
    }
}
