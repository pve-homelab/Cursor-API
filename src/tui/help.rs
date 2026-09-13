//! Scrollable Help content for the Cursor-API TUI.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub fn help_lines() -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    section_title(&mut lines, "1. Overview");
    lines.push(Line::from(Span::styled(
        "Cursor-API",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(
        "Local OpenAI-compatible /v1 proxy backed by the Cursor Agent CLI.",
    ));
    lines.push(Line::from(
        "The TUI starts the HTTP API automatically. Use the CLI tab for an interactive agent shell.",
    ));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Default URL: ", Style::default().fg(Color::DarkGray)),
        Span::raw("http://127.0.0.1:8787/v1"),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Default model: ", Style::default().fg(Color::DarkGray)),
        Span::raw("auto  (see ready banner / /health → default_model)"),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from("Profiles: chat · json_api · long_running"));
    lines.push(Line::from(
        "Env: BRIDGE_HOST BRIDGE_PORT BRIDGE_API_KEY BRIDGE_TIMEOUT_SECS BRIDGE_JSON_MODE BRIDGE_DEFAULT_MODEL CURSOR_WORKSPACE",
    ));
    lines.push(Line::from(""));

    section_title(&mut lines, "2. Cursor-API CLI commands");
    lines.push(cmd_line("cursor-api", "TUI + start HTTP API (default)"));
    lines.push(cmd_line(
        "cursor-api tui --no-autostart",
        "TUI only; press s to start API",
    ));
    lines.push(cmd_line("cursor-api serve", "Headless HTTP only (no TUI)"));
    lines.push(cmd_line("cursor-api doctor", "Locate / probe agent binary"));
    lines.push(cmd_line(
        "cursor-api doctor --full",
        "Probe + live smoke completion",
    ));
    lines.push(cmd_line(
        "cursor-api config-path",
        "Print config.toml location",
    ));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "TUI keys",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(
        "  1–6 / Tab     switch pages (on CLI tab: Ctrl+← / Ctrl+→ or F1–F6)",
    ));
    lines.push(Line::from("  s             start/stop API (Dashboard)"));
    lines.push(Line::from("  r             health-check agent (Dashboard)"));
    lines.push(Line::from("  t             smoke test (Dashboard)"));
    lines.push(Line::from("  q / Esc       quit Cursor-API"));
    lines.push(Line::from(""));

    section_title(&mut lines, "3. /v1 API compatibility");
    lines.push(Line::from(Span::styled(
        "Endpoints",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(api_line("GET", "/health", "Service status, model, usage"));
    lines.push(api_line("GET", "/healthz", "Alias of /health"));
    lines.push(api_line("GET", "/v1/models", "Model list"));
    lines.push(api_line(
        "POST",
        "/v1/chat/completions",
        "Chat (sync or SSE stream)",
    ));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Supported",
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(compat_line(true, "messages[] (system / user / assistant / tool text)"));
    lines.push(compat_line(true, "model (default from config)"));
    lines.push(compat_line(true, "stream: true → SSE chunks + final usage"));
    lines.push(compat_line(
        true,
        "response_format: json_object | json_schema",
    ));
    lines.push(compat_line(true, "X-Request-ID (echoed on responses)"));
    lines.push(compat_line(true, "HTTP 429 when reject_when_busy + full"));
    lines.push(compat_line(true, "Authorization: Bearer <bridge api key>"));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Not supported / ignored",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(compat_line(false, "temperature / top_p / max_tokens"));
    lines.push(compat_line(false, "tool_calls / function calling"));
    lines.push(compat_line(false, "vision / image content parts"));
    lines.push(compat_line(false, "embeddings / audio / assistants API"));
    lines.push(Line::from(""));

    section_title(&mut lines, "4. CLI tab (embedded agent shell)");
    lines.push(Line::from(
        "Opens a real terminal (PowerShell / your $SHELL) with agent on PATH.",
    ));
    lines.push(Line::from(
        "Type agent … as usual: agent --version · agent login · agent -p \"…\"",
    ));
    lines.push(Line::from(
        "Ctrl+L clears the local view hint; type exit to leave the shell process (session restarts).",
    ));
    lines.push(Line::from(""));

    section_title(&mut lines, "5. Safety");
    lines.push(Line::from(
        "Default mode is ask (read-only). Keep force=false unless you want workspace writes.",
    ));
    lines.push(Line::from(
        "Prefer bind 127.0.0.1 and a bridge API key for anything beyond solo local use.",
    ));

    lines
}

fn section_title(lines: &mut Vec<Line<'static>>, title: &str) {
    lines.push(Line::from(Span::styled(
        title.to_string(),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )));
    lines.push(Line::from(""));
}

fn cmd_line(cmd: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {cmd:<34}"),
            Style::default().fg(Color::Green),
        ),
        Span::raw(desc.to_string()),
    ])
}

fn api_line(method: &str, path: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {method:<6}"),
            Style::default().fg(Color::Magenta),
        ),
        Span::styled(
            format!("{path:<28}"),
            Style::default().fg(Color::Green),
        ),
        Span::raw(desc.to_string()),
    ])
}

fn compat_line(ok: bool, text: &str) -> Line<'static> {
    let mark = if ok { "  ✓ " } else { "  ✗ " };
    let color = if ok { Color::Green } else { Color::DarkGray };
    Line::from(vec![
        Span::styled(mark, Style::default().fg(color)),
        Span::raw(text.to_string()),
    ])
}
