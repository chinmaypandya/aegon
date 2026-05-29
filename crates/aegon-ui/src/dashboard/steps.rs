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

/// Draw running tool calls (with spinner) and the last few completed steps.
pub fn draw(f: &mut Frame, area: Rect, state: &SessionState, tick: u64) {
    let spinner = SPINNER[(tick as usize) % SPINNER.len()];

    let mut items: Vec<ListItem> = Vec::new();

    // Running steps first, with spinner animation.
    for step in state.running_steps() {
        let elapsed = if let StepStatus::Running { started_at } = &step.status {
            let ms = (chrono::Utc::now() - started_at).num_milliseconds().max(0);
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

    // Most recent completed steps (newest last, show up to available height).
    let max_completed = (area.height as usize).saturating_sub(items.len() + 2);
    let completed = state.completed_steps();
    let start = completed.len().saturating_sub(max_completed);

    for step in &completed[start..] {
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
            .title(" Steps "),
    );
    f.render_widget(list, area);
}
