//! Causal flow string — renders the session's event chain as a one-line summary.
//!
//! Example output: `USER → THINK → [Bash ‖ Read] → ASST → USER → Write → ASST`

use aegon_core::{FlowNode, SessionState};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

/// Draw the causal flow string in `area`.
pub fn draw(f: &mut Frame, area: Rect, state: &SessionState) {
    let line = build_flow_line(&state.flow);
    let widget = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Flow "),
    );
    f.render_widget(widget, area);
}

fn build_flow_line(nodes: &[FlowNode]) -> Line<'static> {
    // Show the last N nodes that fit on one line.
    let display: Vec<&FlowNode> = nodes
        .iter()
        .rev()
        .take(12)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    let mut spans: Vec<Span> = Vec::new();

    for (i, node) in display.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" → ", Style::default().fg(Color::DarkGray)));
        }

        let (label, color) = match node {
            FlowNode::Human => ("USER", Color::White),
            FlowNode::Thinking => ("THINK", Color::Yellow),
            FlowNode::Assistant => ("ASST", Color::Magenta),
            FlowNode::ToolGroup(names) => {
                let label = if names.len() == 1 {
                    names[0].clone()
                } else {
                    format!("[{}]", names.join(" ‖ "))
                };
                spans.push(Span::styled(
                    label,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ));
                continue;
            }
        };

        spans.push(Span::styled(
            label,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    }

    if spans.is_empty() {
        spans.push(Span::styled(
            "waiting for events…",
            Style::default().fg(Color::DarkGray),
        ));
    }

    Line::from(spans)
}
