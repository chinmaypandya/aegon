//! Terminal UI — renders live [`LogEvent`]s using ratatui.
//!
//! Runs on the caller's thread. Receives events from the watcher over a
//! channel, updates [`App`] state, and redraws on every tick. Press `q` or
//! `Esc` to quit.

use crate::app::App;
use aegon_types::{EventKind, LogEvent};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::io;
use std::sync::mpsc::Receiver;
use std::time::Duration;

/// Enter raw mode, run the event loop, and restore the terminal on exit.
pub fn run(rx: Receiver<LogEvent>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, rx);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    rx: Receiver<LogEvent>,
) -> Result<()> {
    let mut app = App::new();

    loop {
        // Drain all pending events from the watcher without blocking.
        while let Ok(event) = rx.try_recv() {
            if !matches!(event.kind, EventKind::Unknown) {
                app.push(event);
            }
        }

        terminal.draw(|f| draw(f, &app))?;

        // Poll keyboard at ~20 fps.
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                    app.should_quit = true;
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn draw(f: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(f.area());

    // ── Header ──────────────────────────────────────────────────────────────
    let header = Paragraph::new("Aegon — Claude Code session monitor")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        );
    f.render_widget(header, chunks[0]);

    // ── Event list ──────────────────────────────────────────────────────────
    let items: Vec<ListItem> = app
        .events
        .iter()
        .rev()
        .take(chunks[1].height as usize)
        .map(|e| ListItem::new(format_event(e)))
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Events (newest first) "),
    );
    f.render_widget(list, chunks[1]);

    // ── Footer ───────────────────────────────────────────────────────────────
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" q", Style::default().fg(Color::Yellow)),
        Span::raw(" quit  "),
        Span::styled(
            format!(" {} events", app.events.len()),
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    f.render_widget(footer, chunks[2]);
}

fn format_event(e: &LogEvent) -> Line<'static> {
    let ts = e.timestamp.format("%H:%M:%S").to_string();
    let (label, color, detail) = match &e.kind {
        EventKind::ToolCall(tc) => (
            "TOOL▶",
            Color::Green,
            format!("{} {}", tc.name, summarise_input(&tc.input)),
        ),
        EventKind::ToolResult(tr) => ("TOOL◀", Color::Blue, truncate(&tr.content, 80)),
        EventKind::AssistantMessage { content, .. } => {
            ("ASST ", Color::Magenta, truncate(content, 80))
        }
        EventKind::UserMessage { content } => ("USER ", Color::White, truncate(content, 80)),
        EventKind::TokenUsage(u) => (
            "TKNS ",
            Color::DarkGray,
            format!(
                "in={} out={} cache_r={}",
                u.input_tokens, u.output_tokens, u.cache_read_input_tokens
            ),
        ),
        EventKind::Unknown => ("???? ", Color::DarkGray, String::new()),
    };

    Line::from(vec![
        Span::styled(format!("{ts} "), Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{label} "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(detail),
    ])
}

fn truncate(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}

fn summarise_input(v: &serde_json::Value) -> String {
    // Show the first string value found in the input object, or the raw JSON.
    if let Some(obj) = v.as_object() {
        if let Some(first) = obj.values().find_map(|v| v.as_str()) {
            return truncate(first, 60);
        }
    }
    truncate(&v.to_string(), 60)
}
