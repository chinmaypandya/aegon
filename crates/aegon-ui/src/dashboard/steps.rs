//! Active tool-call spinners and recent completed steps.

use aegon_core::{SessionState, StepStatus};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem},
    Frame,
};

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Only show pending steps started within this window as truly "running".
///
/// Beyond this threshold the step is likely orphaned (interrupted session) and
/// should not appear as in-flight to avoid cluttering the panel.
const MAX_RUNNING_AGE_SECS: i64 = 30;

/// Draw running tool calls (with spinner) and the most recent completed steps.
///
/// Completed steps are shown newest-first so the most recent work is always
/// visible without scrolling.
pub fn draw(f: &mut Frame, area: Rect, state: &SessionState, tick: u64) {
    let spinner = SPINNER[(tick as usize) % SPINNER.len()];
    let capacity = area.height.saturating_sub(2) as usize; // subtract border lines
    let mut items: Vec<ListItem> = Vec::new();

    // Thinking placeholder — model is reasoning but the JSONL record hasn't
    // been flushed yet. Gives real-time feedback while the turn is in flight.
    if state.awaiting_response {
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("{spinner} "),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Thinking…   ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ])));
    }

    // Running steps — only those started within the recent window.
    let now = chrono::Utc::now();
    let mut running: Vec<_> = state
        .running_steps()
        .into_iter()
        .filter(|s| {
            if let StepStatus::Running { started_at } = &s.status {
                (now - started_at).num_seconds() <= MAX_RUNNING_AGE_SECS
            } else {
                false
            }
        })
        .collect();
    // Most recently started first.
    running.sort_by(|a, b| {
        let ta = if let StepStatus::Running { started_at } = &a.status {
            *started_at
        } else {
            now
        };
        let tb = if let StepStatus::Running { started_at } = &b.status {
            *started_at
        } else {
            now
        };
        tb.cmp(&ta)
    });

    for step in &running {
        if items.len() >= capacity {
            break;
        }
        let elapsed = if let StepStatus::Running { started_at } = &step.status {
            let ms = (now - started_at).num_milliseconds().max(0);
            format!("{:.1}s", ms as f64 / 1000.0)
        } else {
            String::new()
        };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("{spinner} "),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<12}", step.tool_name),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" [{elapsed}]"),
                Style::default().fg(Color::DarkGray),
            ),
        ])));
    }

    // Completed steps — newest first, fill remaining capacity.
    let remaining = capacity.saturating_sub(items.len());
    let completed = state.completed_steps();

    for step in completed.iter().rev().take(remaining) {
        let (icon, color, timing) = match &step.status {
            StepStatus::Done { duration_ms } => ("✓", Color::Cyan, format!(" {duration_ms}ms")),
            StepStatus::Failed { duration_ms } => ("✗", Color::Red, format!(" {duration_ms}ms")),
            StepStatus::Running { .. } => continue,
        };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(format!("{icon} "), Style::default().fg(color)),
            Span::styled(
                format!("{:<12}", step.tool_name),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(timing, Style::default().fg(Color::DarkGray)),
        ])));
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Steps (newest first) "),
    );
    f.render_widget(list, area);
}
