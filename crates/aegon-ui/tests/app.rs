//! Integration tests for the aegon-ui App state.
//!
//! Verifies the rolling event buffer behaviour from outside the crate.

use aegon_types::{EventKind, LogEvent, StreamId};
use aegon_ui::app::App;
use chrono::Utc;
use uuid::Uuid;

fn dummy_event() -> LogEvent {
    LogEvent {
        id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
        parent_id: None,
        timestamp: Utc::now(),
        stream: StreamId::Main,
        kind: EventKind::UserMessage {
            content: "hi".into(),
        },
    }
}

#[test]
fn new_app_starts_empty() {
    let app = App::new();
    assert!(app.events.is_empty());
    assert!(!app.should_quit);
}

#[test]
fn push_appends_event() {
    let mut app = App::new();
    app.push(dummy_event());
    assert_eq!(app.events.len(), 1);
}

#[test]
fn push_evicts_oldest_when_buffer_full() {
    let mut app = App::new();

    // Fill to capacity (500) + 1 more.
    let first_id = {
        let e = dummy_event();
        let id = e.id;
        app.push(e);
        id
    };
    for _ in 1..500 {
        app.push(dummy_event());
    }
    // At capacity — first event still present.
    assert_eq!(app.events.len(), 500);
    assert_eq!(app.events[0].id, first_id);

    // One more push should evict the oldest.
    app.push(dummy_event());
    assert_eq!(app.events.len(), 500);
    assert_ne!(app.events[0].id, first_id);
}
