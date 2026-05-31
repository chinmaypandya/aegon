//! SSE (Server-Sent Events) stream parser.
//!
//! Accepts raw bytes from a `text/event-stream` response body and yields
//! complete `SseEvent`s as they accumulate. The parser is stateful and
//! call-by-call: feed it chunks as they arrive from the network.
//!
//! Follows the SSE spec (https://html.spec.whatwg.org/#server-sent-events):
//! events are separated by blank lines; each event has optional `event:` and
//! `data:` fields. Multi-line `data:` values are joined with `\n`.

/// A single, fully-received SSE event.
#[derive(Debug, Clone)]
pub struct SseEvent {
    /// The `event:` field value, e.g. `"content_block_delta"`.
    /// Defaults to `"message"` when the field is absent (per spec).
    pub event_type: String,
    /// The concatenated `data:` field value.
    pub data: String,
}

/// Stateful SSE parser that operates on raw byte chunks.
///
/// Create one per response body and call [`SseParser::feed`] as bytes arrive.
/// Each call returns zero or more complete events.
#[derive(Default)]
pub struct SseParser {
    /// Leftover bytes from the previous feed that did not end with a newline.
    remainder: String,
    /// The `event:` field value being accumulated for the current event block.
    current_event: Option<String>,
    /// The `data:` lines being accumulated for the current event block.
    current_data: Vec<String>,
}

impl SseParser {
    /// Feed raw bytes into the parser and return any complete events.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<SseEvent> {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return vec![];
        };

        let mut complete = Vec::new();
        // Prepend any leftover from the previous call.
        let buf = if self.remainder.is_empty() {
            text.to_owned()
        } else {
            let mut s = std::mem::take(&mut self.remainder);
            s.push_str(text);
            s
        };

        let mut lines = buf.split('\n').peekable();
        while let Some(raw_line) = lines.next() {
            // If this is the last segment and there's no trailing newline,
            // it may be an incomplete line — save it for the next feed call.
            if lines.peek().is_none() && !buf.ends_with('\n') {
                self.remainder = raw_line.to_owned();
                break;
            }

            let line = raw_line.trim_end_matches('\r');

            if line.is_empty() {
                // Blank line dispatches the current event block.
                if let Some(data) = self.flush_event() {
                    complete.push(data);
                }
            } else if let Some(value) = line.strip_prefix("event:") {
                self.current_event = Some(value.trim_start().to_owned());
            } else if let Some(value) = line.strip_prefix("data:") {
                self.current_data.push(value.trim_start().to_owned());
            }
            // Lines starting with ':' are comments; all other field names are ignored.
        }

        complete
    }

    /// Flush the accumulated fields into an `SseEvent`, then reset state.
    /// Returns `None` when no `data:` field was present (comment-only blocks).
    fn flush_event(&mut self) -> Option<SseEvent> {
        if self.current_data.is_empty() {
            self.current_event = None;
            return None;
        }
        let event = SseEvent {
            event_type: self
                .current_event
                .take()
                .unwrap_or_else(|| "message".into()),
            data: self.current_data.join("\n"),
        };
        self.current_data.clear();
        Some(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_event() {
        let mut p = SseParser::default();
        let input = b"event: content_block_delta\ndata: {\"type\":\"x\"}\n\n";
        let events = p.feed(input);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "content_block_delta");
        assert_eq!(events[0].data, "{\"type\":\"x\"}");
    }

    #[test]
    fn parses_two_events_in_one_chunk() {
        let mut p = SseParser::default();
        let input = b"event: ping\ndata: {}\n\nevent: msg\ndata: hi\n\n";
        let events = p.feed(input);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "ping");
        assert_eq!(events[1].event_type, "msg");
    }

    #[test]
    fn handles_split_across_chunks() {
        let mut p = SseParser::default();
        // First chunk ends mid-line.
        let a = p.feed(b"event: foo\ndat");
        assert!(a.is_empty());
        let b_events = p.feed(b"a: bar\n\n");
        assert_eq!(b_events.len(), 1);
        assert_eq!(b_events[0].data, "bar");
    }

    #[test]
    fn multiline_data_joined_with_newline() {
        let mut p = SseParser::default();
        let input = b"data: line1\ndata: line2\n\n";
        let events = p.feed(input);
        assert_eq!(events[0].data, "line1\nline2");
    }

    #[test]
    fn comment_only_block_produces_no_event() {
        let mut p = SseParser::default();
        let input = b": just a comment\n\n";
        let events = p.feed(input);
        assert!(events.is_empty());
    }

    #[test]
    fn default_event_type_is_message() {
        let mut p = SseParser::default();
        let input = b"data: hello\n\n";
        let events = p.feed(input);
        assert_eq!(events[0].event_type, "message");
    }
}
