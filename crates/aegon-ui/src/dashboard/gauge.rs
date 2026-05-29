//! Token usage gauge — last-turn bar with cumulative session totals.

use aegon_core::SessionState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
    Frame,
};

/// Context window limit used for the per-turn gauge denominator.
const CONTEXT_LIMIT: u64 = 200_000;

/// Draw the token gauge panel.
///
/// The **bar** shows the last turn's (input + output) against the context
/// limit — this is the meaningful per-turn signal. The label below shows
/// cumulative session totals and estimated cost.
pub fn draw(f: &mut Frame, area: Rect, state: &SessionState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    // ── Per-turn bar ─────────────────────────────────────────────────────────
    let (bar_ratio, bar_label) = match &state.last_turn_usage {
        Some(u) => {
            let used = (u.input_tokens + u.output_tokens).min(CONTEXT_LIMIT);
            let ratio = used as f64 / CONTEXT_LIMIT as f64;
            let label = format!(
                "last turn  in={} out={}  cache_r={}",
                u.input_tokens, u.output_tokens, u.cache_read_input_tokens,
            );
            (ratio, label)
        }
        None => (0.0, "no turns yet".into()),
    };

    let bar_color = match bar_ratio {
        r if r < 0.5 => Color::Green,
        r if r < 0.8 => Color::Yellow,
        _ => Color::Red,
    };

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::TOP)
                .border_type(BorderType::Rounded)
                .title(" Tokens (last turn) "),
        )
        .gauge_style(Style::default().fg(bar_color))
        .ratio(bar_ratio)
        .label(bar_label);
    f.render_widget(gauge, chunks[0]);

    // ── Cumulative session totals ─────────────────────────────────────────────
    let t = &state.token_totals;
    let totals = Paragraph::new(format!(
        " session total  in={}  out={}  cache_r={}",
        t.input, t.output, t.cache_read,
    ))
    .block(
        Block::default()
            .borders(Borders::LEFT | Borders::RIGHT)
            .border_type(BorderType::Rounded),
    )
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(totals, chunks[1]);

    // ── Estimated cost ────────────────────────────────────────────────────────
    let cost = Paragraph::new(format!(" est. cost: ${:.4}", t.estimated_cost_usd()))
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                .border_type(BorderType::Rounded),
        )
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(cost, chunks[2]);
}
