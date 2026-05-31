//! Terminal UI — split view: raw event feed (left) + dashboard (right).
//!
//! Runs on the caller's thread. Receives events from the watcher over a
//! channel, updates [`App`] state, and redraws on every tick. Press `q` or
//! `Esc` to quit.

use crate::app::App;
use crate::dashboard;
use aegon_types::{EventKind, LogEvent, StreamId};
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
        app.tick();

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
    // ── Outer layout: header / body / footer ────────────────────────────────
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(f.area());

    // Header bar
    let sessions_label = format!(
        " Aegon   {} sessions   {} events ",
        app.sessions.len(),
        app.events.len()
    );
    f.render_widget(
        Paragraph::new(sessions_label).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        outer[0],
    );

    // ── Body: raw feed (left 50%) | dashboard (right 50%) ───────────────────
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(outer[1]);

    draw_feed(f, body[0], app);
    draw_dashboard_panel(f, body[1], app);

    // Footer
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" q", Style::default().fg(Color::Yellow)),
            Span::raw(" quit"),
        ])),
        outer[2],
    );
}

/// Left panel: raw event feed newest-first.
fn draw_feed(f: &mut ratatui::Frame, area: ratatui::layout::Rect, app: &App) {
    let avail = area.width.saturating_sub(22) as usize; // subtract label+timestamp prefix
    let items: Vec<ListItem> = app
        .events
        .iter()
        .rev()
        .take(area.height as usize)
        .map(|e| ListItem::new(format_event(e, avail)))
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Events (newest first) "),
    );
    f.render_widget(list, area);
}

/// Right panel: aggregated dashboard for the latest active session.
fn draw_dashboard_panel(f: &mut ratatui::Frame, area: ratatui::layout::Rect, app: &App) {
    match app.sessions.latest() {
        Some(state) => dashboard::draw(f, area, state, app.tick),
        None => {
            let placeholder = Paragraph::new("Waiting for session events…")
                .style(Style::default().fg(Color::DarkGray))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title(" Dashboard "),
                );
            f.render_widget(placeholder, area);
        }
    }
}

fn format_event(e: &LogEvent, max_detail: usize) -> Line<'static> {
    let ts = e.timestamp.format("%H:%M:%S").to_string();
    let (label, color, detail) = match &e.kind {
        // ── core conversation ────────────────────────────────────────────────
        EventKind::ToolCall(tc) => (
            "TOOL▶",
            Color::Green,
            format!("{} {}", tc.name, summarise_input(&tc.input)),
        ),
        EventKind::ToolResult(tr) => ("TOOL◀", Color::Blue, truncate(&tr.content, max_detail)),
        EventKind::AssistantMessage { content, .. } => {
            ("ASST ", Color::Magenta, truncate(content, max_detail))
        }
        EventKind::UserMessage { content } => {
            ("USER ", Color::White, truncate(content, max_detail))
        }
        EventKind::TokenUsage(u) => (
            "TKNS ",
            Color::DarkGray,
            format!(
                "in={} out={} cache_r={}",
                u.input_tokens, u.output_tokens, u.cache_read_input_tokens
            ),
        ),
        EventKind::Thinking { text, .. } => ("THINK", Color::Yellow, truncate(text, max_detail)),
        EventKind::SessionTitle { title } => ("TITLE", Color::Cyan, truncate(title, max_detail)),
        EventKind::SessionMode { mode } => ("MODE ", Color::DarkGray, mode.clone()),
        EventKind::SystemError {
            message,
            code,
            retry_attempt,
            max_retries,
            ..
        } => {
            let base = match code {
                Some(c) => format!("[{c}] {}", truncate(message, max_detail.saturating_sub(10))),
                None => truncate(message, max_detail),
            };
            let retry_suffix = match (retry_attempt, max_retries) {
                (Some(attempt), Some(max)) => format!(" (retry {attempt}/{max})"),
                (Some(attempt), None) => format!(" (retry #{attempt})"),
                _ => String::new(),
            };
            ("ERR  ", Color::Red, format!("{base}{retry_suffix}"))
        }

        // ── session bookkeeping ──────────────────────────────────────────────
        EventKind::QueueOperation { operation } => ("QUEUE", Color::DarkGray, operation.clone()),
        EventKind::PrLinked {
            pr_number,
            pr_url,
            repository,
        } => (
            "PR   ",
            Color::Cyan,
            format!(
                "#{pr_number} {repository} {}",
                truncate(pr_url, max_detail.saturating_sub(20))
            ),
        ),
        EventKind::LastPrompt { content } => {
            ("LAST ", Color::DarkGray, truncate(content, max_detail))
        }
        EventKind::FileSnapshot { is_update } => (
            "SNAP ",
            Color::DarkGray,
            if *is_update {
                "update".into()
            } else {
                "initial".into()
            },
        ),

        // ── harness attachment events ────────────────────────────────────────
        EventKind::ToolsRegistered { added, removed } => {
            let detail = if removed.is_empty() {
                format!("+{} tool(s): {}", added.len(), added.join(", "))
            } else {
                format!(
                    "+{} -{} tool(s): {}",
                    added.len(),
                    removed.len(),
                    added.join(", ")
                )
            };
            ("TOOLS", Color::Yellow, truncate(&detail, max_detail))
        }
        EventKind::SkillsLoaded { content } => {
            // Count skills by counting "- " prefixes.
            let count = content.lines().filter(|l| l.starts_with("- ")).count();
            (
                "SKILL",
                Color::Yellow,
                format!("{count} skill(s) available"),
            )
        }
        EventKind::PlanModeEntered { plan_file, .. } => {
            let file = plan_file
                .as_deref()
                .and_then(|p| p.rsplit('/').next())
                .unwrap_or("plan");
            (
                "PLAN▶",
                Color::Magenta,
                format!("entered plan mode ({file})"),
            )
        }
        EventKind::PlanModeExited { plan_file } => {
            let file = plan_file
                .as_deref()
                .and_then(|p| p.rsplit('/').next())
                .unwrap_or("plan");
            (
                "PLAN■",
                Color::DarkGray,
                format!("exited plan mode ({file})"),
            )
        }
        EventKind::TodoUpdated { item_count } => {
            ("TODO ", Color::DarkGray, format!("{item_count} item(s)"))
        }
        EventKind::HookOutput {
            hook_name, content, ..
        } => {
            let first = content.first().map(String::as_str).unwrap_or("");
            (
                "HOOK ",
                Color::Blue,
                format!(
                    "{hook_name}: {}",
                    truncate(first, max_detail.saturating_sub(20))
                ),
            )
        }
        EventKind::FileEdited { path, .. } => {
            let filename = path.rsplit('/').next().unwrap_or(path.as_str());
            ("EDIT ", Color::Green, filename.to_owned())
        }
        EventKind::DateChange { new_date } => ("DATE ", Color::DarkGray, format!("→ {new_date}")),
        EventKind::BackgroundTaskResult {
            task_id,
            status,
            summary,
        } => {
            let id_prefix = task_id
                .as_deref()
                .map(|id| format!("[{id}] "))
                .unwrap_or_default();
            let color = if status == "completed" {
                Color::Green
            } else {
                Color::Red
            };
            (
                "BGTSK",
                color,
                format!(
                    "{id_prefix}{status}: {}",
                    truncate(summary, max_detail.saturating_sub(30))
                ),
            )
        }
        EventKind::PermissionsUpdated { allowed_tools } => (
            "PERMS",
            Color::DarkGray,
            format!("{} tool(s) allowed", allowed_tools.len()),
        ),

        EventKind::Unknown => ("???? ", Color::DarkGray, String::new()),
    };

    // Prefix sidechain events so they are visually subordinate to the main chain.
    let stream_prefix = if e.stream == StreamId::Sidechain {
        "[S] "
    } else {
        ""
    };

    Line::from(vec![
        Span::styled(format!("{ts} "), Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{stream_prefix}{label} "),
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
