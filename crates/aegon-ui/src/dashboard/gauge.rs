//! Token usage gauge — shows consumed tokens against an estimated context limit.

use aegon_core::SessionState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
    Frame,
};

/// Approximate context window used as the gauge denominator.
///
/// Claude models vary (100k–200k+); 200k is a safe upper bound that keeps
/// the bar from saturating on typical sessions.
const CONTEXT_LIMIT_TOKENS: u64 = 200_000;

/// Draw the token progress bar and cost estimate.
pub fn draw(f: &mut Frame, area: Rect, state: &SessionState) {
    let t = &state.token_totals;
    let used = t.total().min(CONTEXT_LIMIT_TOKENS);
    let ratio = used as f64 / CONTEXT_LIMIT_TOKENS as f64;

    let bar_color = match ratio {
        r if r < 0.5 => Color::Green,
        r if r < 0.8 => Color::Yellow,
        _ => Color::Red,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(1)])
        .split(area);

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::TOP)
                .border_type(BorderType::Rounded)
                .title(" Tokens "),
        )
        .gauge_style(Style::default().fg(bar_color))
        .ratio(ratio)
        .label(format!(
            "{:.0}k / {:.0}k   in={} out={} cache_r={}",
            used as f64 / 1000.0,
            CONTEXT_LIMIT_TOKENS as f64 / 1000.0,
            t.input,
            t.output,
            t.cache_read,
        ));
    f.render_widget(gauge, chunks[0]);

    let cost = Paragraph::new(format!(" est. cost: ${:.4}", t.estimated_cost_usd()))
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                .border_type(BorderType::Rounded),
        )
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(cost, chunks[1]);
}
