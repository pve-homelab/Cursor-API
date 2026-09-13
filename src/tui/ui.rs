use super::app::{ConfigField, Tab, TuiApp};
use crate::log_buffer::LogLevel;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap};
use ratatui::Frame;

pub fn draw(frame: &mut Frame, app: &TuiApp) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(frame.area());

    draw_header(frame, root[0], app);
    match app.tab {
        Tab::Dashboard => draw_dashboard(frame, root[1], app),
        Tab::Config => draw_config(frame, root[1], app),
        Tab::Cli => draw_cli(frame, root[1], app),
        Tab::Logs => draw_logs(frame, root[1], app),
        Tab::Usage => draw_usage(frame, root[1], app),
        Tab::Help => draw_help(frame, root[1], app),
    }
    draw_footer(frame, root[2], app);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .map(|t| Line::from(format!(" {} ", t.title())))
        .collect();
    let selected = Tab::ALL.iter().position(|t| *t == app.tab).unwrap_or(0);
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Cursor-API "),
        )
        .select(selected)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, area);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let hint = match app.tab {
        Tab::Dashboard => "s start/stop · r health · t smoke · 1-6 tabs · q quit",
        Tab::Config => "↑/↓ · Enter edit · p profile · j json · w force · k trust",
        Tab::Cli => "keys → shell · Shift+R restart · F1–F6 / Ctrl+←→ tabs · Ctrl+Q quit",
        Tab::Logs => "↑/↓ scroll · c clear",
        Tab::Usage => "↑/↓ scroll · x reset",
        Tab::Help => "↑/↓ PgUp/PgDn scroll · Home/End · 1-6 tabs",
    };
    let text = if app.editing {
        format!("Editing {}: {}▌", app.config_field.label(), app.edit_buffer)
    } else if app.smoke_running {
        format!("Smoke test running…  |  {}", hint)
    } else {
        format!("{}  |  {}", app.status_message, hint)
    };
    let p = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(" status "))
        .wrap(Wrap { trim: true });
    frame.render_widget(p, area);
}

fn draw_dashboard(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let cfg = app.state.config.read().clone();
    let running = app.state.is_running();
    let healthy = app.state.is_healthy();
    let agent = app
        .state
        .agent_version
        .read()
        .clone()
        .unwrap_or_else(|| "unknown".into());
    let usage = app.state.usage.totals();
    let last_error = app.state.last_error.read().clone();
    let binary = app
        .state
        .backend
        .resolve_launch()
        .map(|l| l.display)
        .unwrap_or_else(|e| format!("MISSING ({e})"));

    let status_color = if running && healthy {
        Color::Green
    } else if running {
        Color::Yellow
    } else {
        Color::Red
    };

    let lines = vec![
        Line::from(vec![
            Span::raw("Service: "),
            Span::styled(
                if running { "RUNNING" } else { "STOPPED" },
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   Health: "),
            Span::styled(
                if healthy { "OK" } else { "DOWN" },
                Style::default().fg(status_color),
            ),
            Span::raw(format!("   Uptime: {}s", app.state.uptime_secs())),
        ]),
        Line::from(""),
        Line::from(format!("Listen:     {}", cfg.listen_addr())),
        Line::from(format!("Base URL:   {}", cfg.base_url())),
        Line::from(format!("OpenAI /v1: {}", cfg.v1_url())),
        Line::from(format!(
            "Profile:    {}   json_mode={}   flatten={}",
            cfg.cursor.profile, cfg.cursor.json_mode, cfg.cursor.message_flatten_mode
        )),
        Line::from(""),
        Line::from(format!("Agent bin:  {binary}")),
        Line::from(format!("Agent ver:  {agent}")),
        Line::from(format!(
            "Mode:       {}   model={}   force={}   trust={}",
            cfg.cursor.mode,
            cfg.cursor.default_model,
            cfg.cursor.force,
            cfg.cursor.trust
        )),
        Line::from(format!(
            "Concurrency max={} reject_when_busy={} timeout={}s",
            cfg.server.max_concurrency, cfg.server.reject_when_busy, cfg.server.request_timeout_secs
        )),
        Line::from(""),
        Line::from(format!(
            "Active req: {}   Totals: {} ok / {} err / {} calls   avg {}ms",
            app.state.usage.active(),
            usage.successes,
            usage.failures,
            usage.requests,
            usage.avg_latency_ms
        )),
        Line::from(format!(
            "Tokens≈     prompt {} · completion {} · total {}",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        )),
        Line::from(format!(
            "Last error: {}",
            last_error.unwrap_or_else(|| "(none)".into())
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Point any OpenAI SDK client at the /v1 URL. Send X-Request-ID to correlate logs.",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Cursor-API · service dashboard "),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

fn draw_config(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let cfg = app.state.config.read().clone();
    let path = app.state.config_path.read().clone();
    let items: Vec<ListItem> = ConfigField::ALL
        .iter()
        .map(|field| {
            let value = match field {
                ConfigField::Host => cfg.server.host.clone(),
                ConfigField::Port => cfg.server.port.to_string(),
                ConfigField::Profile => cfg.cursor.profile.clone(),
                ConfigField::Model => cfg.cursor.default_model.clone(),
                ConfigField::Mode => cfg.cursor.mode.clone(),
                ConfigField::Workspace => {
                    if cfg.cursor.workspace.is_empty() {
                        "(process cwd)".into()
                    } else {
                        cfg.cursor.workspace.clone()
                    }
                }
                ConfigField::FlattenMode => cfg.cursor.message_flatten_mode.clone(),
                ConfigField::JsonMode => cfg.cursor.json_mode.to_string(),
                ConfigField::ApiKey => mask_secret(&cfg.auth.api_key),
                ConfigField::CursorApiKey => mask_secret(&cfg.cursor.cursor_api_key),
                ConfigField::MaxConcurrency => cfg.server.max_concurrency.to_string(),
                ConfigField::Timeout => cfg.server.request_timeout_secs.to_string(),
                ConfigField::RejectWhenBusy => cfg.server.reject_when_busy.to_string(),
            };
            let selected = *field == app.config_field;
            let style = if selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let marker = if selected { "▶" } else { " " };
            ListItem::new(Line::from(Span::styled(
                format!("{marker} {:<28} {}", field.label(), value),
                style,
            )))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" configuration · {} ", path.display())),
    );
    frame.render_widget(list, area);
}

fn mask_secret(value: &str) -> String {
    if value.is_empty() {
        "(empty)".into()
    } else if value.len() <= 4 {
        "****".into()
    } else {
        format!("{}…{}", &value[..2], &value[value.len() - 2..])
    }
}

fn draw_logs(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let entries = app.state.logs.snapshot();
    let height = area.height.saturating_sub(2) as usize;
    let total = entries.len();
    let scroll = app.log_scroll as usize;
    let end = total.saturating_sub(scroll);
    let start = end.saturating_sub(height);
    let slice = if start < end {
        &entries[start..end]
    } else {
        &[]
    };

    let items: Vec<ListItem> = slice
        .iter()
        .map(|e| {
            let color = match e.level {
                LogLevel::Error => Color::Red,
                LogLevel::Warn => Color::Yellow,
                LogLevel::Info => Color::Green,
                LogLevel::Debug => Color::Blue,
                LogLevel::Trace => Color::DarkGray,
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{} ", e.time.format("%H:%M:%S")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{:<5} ", e.level.as_str()),
                    Style::default().fg(color),
                ),
                Span::raw(e.message.clone()),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" logs ({total}) ")),
    );
    frame.render_widget(list, area);
}

fn draw_usage(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(3)])
        .split(area);

    let totals = app.state.usage.totals();
    let summary = Paragraph::new(vec![
        Line::from(format!(
            "Requests: {}   Success: {}   Failures: {}   Active: {}",
            totals.requests,
            totals.successes,
            totals.failures,
            app.state.usage.active()
        )),
        Line::from(format!(
            "Prompt tokens≈ {}   Completion≈ {}   Total≈ {}   Avg latency {}ms",
            totals.prompt_tokens, totals.completion_tokens, totals.total_tokens, totals.avg_latency_ms
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Token counts are estimates (chars/4). Response previews shown for completed requests.",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(Block::default().borders(Borders::ALL).title(" totals "));
    frame.render_widget(summary, chunks[0]);

    let recent = app.state.usage.recent();
    let height = chunks[1].height.saturating_sub(2) as usize;
    let scroll = app.usage_scroll as usize;
    let end = recent.len().saturating_sub(scroll);
    let start = end.saturating_sub(height);
    let slice = if start < end { &recent[start..end] } else { &[] };

    let items: Vec<ListItem> = slice
        .iter()
        .map(|r| {
            let color = match r.status.as_str() {
                "ok" => Color::Green,
                "error" => Color::Red,
                _ => Color::Yellow,
            };
            let preview = r
                .response_preview
                .clone()
                .unwrap_or_else(|| r.error.clone().unwrap_or_default());
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!("{} ", r.started_at.format("%H:%M:%S")),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(format!("{:<7} ", r.status), Style::default().fg(color)),
                    Span::raw(format!(
                        "req={} model={} p≈{} c≈{} {}ms",
                        r.request_id, r.model, r.prompt_tokens, r.completion_tokens, r.duration_ms
                    )),
                ]),
                Line::from(Span::styled(
                    preview,
                    Style::default().fg(Color::DarkGray),
                )),
            ])
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" recent requests "),
    );
    frame.render_widget(list, chunks[1]);
}

fn draw_cli(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    let alive = app.cli.as_ref().is_some_and(|c| c.alive());
    let banner = if alive {
        "Embedded shell (PowerShell / $SHELL). Run agent like a normal terminal. Try: agent --version"
    } else {
        "Shell not running — press r to start, or Shift+R to restart"
    };
    let banner_p = Paragraph::new(banner)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Cursor-API · CLI "),
        )
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(banner_p, chunks[0]);

    let lines: Vec<Line> = if let Some(cli) = app.cli.as_ref() {
        cli.screen_lines()
            .into_iter()
            .map(|s| Line::from(s))
            .collect()
    } else {
        vec![Line::from("Starting embedded CLI…")]
    };

    let term = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(if alive {
                    " terminal "
                } else {
                    " terminal (dead) "
                }),
        )
        .style(Style::default().fg(Color::Green));
    frame.render_widget(term, chunks[1]);
}

fn draw_help(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let lines = super::help::help_lines();
    let total = lines.len();
    let height = area.height.saturating_sub(2) as usize;
    let max_scroll = total.saturating_sub(height.max(1));
    let scroll = (app.help_scroll as usize).min(max_scroll);

    let p = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(format!(
            " Cursor-API · help  (lines {}–{} of {}) ",
            scroll + 1,
            (scroll + height).min(total).max(scroll + 1),
            total
        )))
        .wrap(Wrap { trim: false })
        .scroll((scroll as u16, 0));
    frame.render_widget(p, area);
}
