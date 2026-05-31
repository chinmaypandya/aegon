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

    let mode_badge = match state.mode.as_deref() {
        Some("auto") => "  [auto]",
        _ => "",
    };

    let plan_badge = if state.in_plan_mode { "  [plan]" } else { "" };

    let error_badge = if state.errors.is_empty() {
        String::new()
    } else {
        let retry_suffix = if state.retry_count > 0 {
            format!(
                ", {} retr{}",
                state.retry_count,
                if state.retry_count == 1 { "y" } else { "ies" }
            )
        } else {
            String::new()
        };
        format!("  ⚠ {} error(s){}", state.errors.len(), retry_suffix)
    };

    let pr_badge = if state.pr_links.is_empty() {
        String::new()
    } else {
        // Show the most recent PR number.
        let (num, _) = &state.pr_links[state.pr_links.len() - 1];
        format!("  PR #{num}")
    };

    let bgtask_badge = if state.background_tasks_completed > 0 {
        format!("  {} bg task(s)", state.background_tasks_completed)
    } else {
        String::new()
    };

    let text = ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(
            format!(" {title}"),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        ratatui::text::Span::styled(mode_badge, Style::default().fg(Color::Cyan)),
        ratatui::text::Span::styled(plan_badge, Style::default().fg(Color::Magenta)),
        ratatui::text::Span::styled(format!("  {status}"), Style::default().fg(status_color)),
        ratatui::text::Span::styled(
            format!("   {} events", state.event_count),
            Style::default().fg(Color::DarkGray),
        ),
        ratatui::text::Span::styled(error_badge, Style::default().fg(Color::Red)),
        ratatui::text::Span::styled(pr_badge, Style::default().fg(Color::Cyan)),
        ratatui::text::Span::styled(bgtask_badge, Style::default().fg(Color::DarkGray)),
    ]);

    let widget = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Session "),
    );
    f.render_widget(widget, area);
}
