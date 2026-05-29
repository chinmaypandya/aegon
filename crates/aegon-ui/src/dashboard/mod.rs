//! Dashboard panel — aggregated live view of the current session.
//!
//! Renders token gauges, active tool spinners, and a causal flow string
//! alongside the raw event feed. Consumes [`SessionState`] produced by
//! `aegon-core`; has no knowledge of JSONL or file watching.

mod flow;
mod gauge;
mod steps;

use aegon_core::SessionState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

/// Draw the full dashboard into `area`.
///
/// `tick` increments each frame and drives the spinner animation on active
/// tool calls.
pub fn draw(f: &mut Frame, area: Rect, state: &SessionState, tick: u64) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // session header
            Constraint::Length(6), // token gauge (bar + totals + cost)
            Constraint::Min(6),    // active steps
            Constraint::Length(3), // flow string
        ])
        .split(area);

    draw_header(f, chunks[0], state);
    gauge::draw(f, chunks[1], state);
    steps::draw(f, chunks[2], state, tick);
    flow::draw(f, chunks[3], state);
}

fn draw_header(f: &mut Frame, area: Rect, state: &SessionState) {
    let title = state.title.as_deref().unwrap_or("Live session");

    let status = if state.is_active() {
        " ● LIVE"
    } else {
        " ○ idle"
    };
    let status_color = if state.is_active() {
        Color::Green
    } else {
        Color::DarkGray
    };

    let text = ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(
            format!(" {title}"),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        ratatui::text::Span::styled(format!("  {status}"), Style::default().fg(status_color)),
        ratatui::text::Span::styled(
            format!("   {} events", state.event_count),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let widget = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Session "),
    );
    f.render_widget(widget, area);
}
