use super::cli_term::CliTerminal;
use crate::server::{start_in_background, stop_server};
use crate::state::AppState;
use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use futures_util::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    Config,
    Cli,
    Logs,
    Usage,
    Help,
}

impl Tab {
    pub const ALL: [Tab; 6] = [
        Tab::Dashboard,
        Tab::Config,
        Tab::Cli,
        Tab::Logs,
        Tab::Usage,
        Tab::Help,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Config => "Config",
            Tab::Cli => "CLI",
            Tab::Logs => "Logs",
            Tab::Usage => "Usage",
            Tab::Help => "Help",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Tab::Dashboard => Tab::Config,
            Tab::Config => Tab::Cli,
            Tab::Cli => Tab::Logs,
            Tab::Logs => Tab::Usage,
            Tab::Usage => Tab::Help,
            Tab::Help => Tab::Dashboard,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Tab::Dashboard => Tab::Help,
            Tab::Config => Tab::Dashboard,
            Tab::Cli => Tab::Config,
            Tab::Logs => Tab::Cli,
            Tab::Usage => Tab::Logs,
            Tab::Help => Tab::Usage,
        }
    }

    pub fn from_index(i: usize) -> Option<Self> {
        Self::ALL.get(i).copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigField {
    Host,
    Port,
    Profile,
    Model,
    Mode,
    Workspace,
    FlattenMode,
    JsonMode,
    ApiKey,
    CursorApiKey,
    MaxConcurrency,
    Timeout,
    RejectWhenBusy,
}

impl ConfigField {
    pub const ALL: [ConfigField; 13] = [
        ConfigField::Host,
        ConfigField::Port,
        ConfigField::Profile,
        ConfigField::Model,
        ConfigField::Mode,
        ConfigField::Workspace,
        ConfigField::FlattenMode,
        ConfigField::JsonMode,
        ConfigField::ApiKey,
        ConfigField::CursorApiKey,
        ConfigField::MaxConcurrency,
        ConfigField::Timeout,
        ConfigField::RejectWhenBusy,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Host => "Bind host",
            Self::Port => "Port",
            Self::Profile => "Profile (chat/json_api/long_running)",
            Self::Model => "Default model",
            Self::Mode => "Mode (ask/agent/plan)",
            Self::Workspace => "Workspace",
            Self::FlattenMode => "Flatten mode",
            Self::JsonMode => "JSON mode",
            Self::ApiKey => "Bridge API key",
            Self::CursorApiKey => "Cursor API key",
            Self::MaxConcurrency => "Max concurrency",
            Self::Timeout => "Timeout (secs)",
            Self::RejectWhenBusy => "Reject when busy (429)",
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|f| *f == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|f| *f == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

pub struct TuiApp {
    pub state: AppState,
    pub tab: Tab,
    pub config_field: ConfigField,
    pub editing: bool,
    pub edit_buffer: String,
    pub log_scroll: u16,
    pub usage_scroll: u16,
    pub help_scroll: u16,
    pub status_message: String,
    pub should_quit: bool,
    pub smoke_running: bool,
    pub cli: Option<CliTerminal>,
    pub cli_cols: u16,
    pub cli_rows: u16,
}

impl TuiApp {
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            tab: Tab::Dashboard,
            config_field: ConfigField::Host,
            editing: false,
            edit_buffer: String::new(),
            log_scroll: 0,
            usage_scroll: 0,
            help_scroll: 0,
            status_message: "Cursor-API · s start/stop · 3 CLI · 6 Help · q quit".into(),
            should_quit: false,
            smoke_running: false,
            cli: None,
            cli_cols: 80,
            cli_rows: 24,
        }
    }

    pub fn ensure_cli(&mut self) {
        if self.cli.as_ref().is_some_and(|c| c.alive()) {
            return;
        }
        let cwd = {
            let cfg = self.state.config.read();
            if cfg.cursor.workspace.is_empty() {
                None
            } else {
                Some(cfg.cursor.workspace.clone())
            }
        };
        match CliTerminal::start(self.cli_cols, self.cli_rows, cwd.as_deref()) {
            Ok(term) => {
                self.status_message =
                    "CLI ready — type agent commands. F1–F6 or Ctrl+←/→ switch tabs.".into();
                self.cli = Some(term);
            }
            Err(err) => {
                self.status_message = format!("CLI start failed: {err:#}");
                self.state.logs.error(format!("CLI pty failed: {err:#}"));
            }
        }
    }

    pub fn tick_cli(&mut self) {
        if let Some(cli) = self.cli.as_mut() {
            cli.poll();
            if !cli.alive() {
                self.status_message =
                    "CLI shell exited — press R on CLI tab to restart.".into();
            }
        }
    }

    fn begin_edit(&mut self) {
        let cfg = self.state.config.read().clone();
        self.edit_buffer = match self.config_field {
            ConfigField::Host => cfg.server.host,
            ConfigField::Port => cfg.server.port.to_string(),
            ConfigField::Profile => cfg.cursor.profile,
            ConfigField::Model => cfg.cursor.default_model,
            ConfigField::Mode => cfg.cursor.mode,
            ConfigField::Workspace => cfg.cursor.workspace,
            ConfigField::FlattenMode => cfg.cursor.message_flatten_mode,
            ConfigField::JsonMode => cfg.cursor.json_mode.to_string(),
            ConfigField::ApiKey => cfg.auth.api_key,
            ConfigField::CursorApiKey => cfg.cursor.cursor_api_key,
            ConfigField::MaxConcurrency => cfg.server.max_concurrency.to_string(),
            ConfigField::Timeout => cfg.server.request_timeout_secs.to_string(),
            ConfigField::RejectWhenBusy => cfg.server.reject_when_busy.to_string(),
        };
        self.editing = true;
    }

    fn apply_edit(&mut self) -> Result<()> {
        {
            let mut cfg = self.state.config.write();
            match self.config_field {
                ConfigField::Host => cfg.server.host = self.edit_buffer.trim().to_string(),
                ConfigField::Port => {
                    cfg.server.port = self.edit_buffer.trim().parse().unwrap_or(cfg.server.port);
                }
                ConfigField::Profile => {
                    cfg.set_profile(self.edit_buffer.trim());
                }
                ConfigField::Model => {
                    cfg.cursor.default_model = self.edit_buffer.trim().to_string();
                }
                ConfigField::Mode => {
                    cfg.cursor.mode = self.edit_buffer.trim().to_string();
                }
                ConfigField::Workspace => {
                    cfg.cursor.workspace = self.edit_buffer.trim().to_string();
                }
                ConfigField::FlattenMode => {
                    cfg.cursor.message_flatten_mode = self.edit_buffer.trim().to_string();
                }
                ConfigField::JsonMode => {
                    cfg.cursor.json_mode = matches!(
                        self.edit_buffer.trim().to_lowercase().as_str(),
                        "1" | "true" | "yes" | "on"
                    );
                }
                ConfigField::ApiKey => {
                    cfg.auth.api_key = self.edit_buffer.trim().to_string();
                    cfg.auth.require_auth = !cfg.auth.api_key.is_empty();
                }
                ConfigField::CursorApiKey => {
                    cfg.cursor.cursor_api_key = self.edit_buffer.trim().to_string();
                }
                ConfigField::MaxConcurrency => {
                    cfg.server.max_concurrency = self
                        .edit_buffer
                        .trim()
                        .parse()
                        .unwrap_or(cfg.server.max_concurrency);
                }
                ConfigField::Timeout => {
                    cfg.server.request_timeout_secs = self
                        .edit_buffer
                        .trim()
                        .parse()
                        .unwrap_or(cfg.server.request_timeout_secs);
                }
                ConfigField::RejectWhenBusy => {
                    cfg.server.reject_when_busy = matches!(
                        self.edit_buffer.trim().to_lowercase().as_str(),
                        "1" | "true" | "yes" | "on"
                    );
                }
            }
            let path = self.state.config_path.read().clone();
            cfg.save(&path)?;
        }
        self.state.reload_backend_from_config();
        self.editing = false;
        self.status_message = format!("Saved {}", self.config_field.label());
        self.state
            .logs
            .info(format!("config updated: {}", self.config_field.label()));
        Ok(())
    }

    fn cycle_profile(&mut self) {
        let mut cfg = self.state.config.write();
        let next = match cfg.cursor.profile.as_str() {
            "chat" => "json_api",
            "json_api" | "json-api" | "json" => "long_running",
            _ => "chat",
        };
        cfg.set_profile(next);
        let path = self.state.config_path.read().clone();
        let _ = cfg.save(&path);
        self.status_message = format!("Profile set to {next}");
        drop(cfg);
        self.state.reload_backend_from_config();
    }

    async fn run_smoke_test(&mut self) {
        if self.smoke_running {
            self.status_message = "Smoke test already running".into();
            return;
        }
        self.smoke_running = true;
        self.status_message = "Running smoke test…".into();
        let backend = self.state.backend.clone();
        let timeout = self
            .state
            .config
            .read()
            .server
            .request_timeout_secs
            .min(120);
        match crate::cursor::smoke_test(&backend, timeout).await {
            Ok(text) => {
                let preview: String = text.chars().take(80).collect();
                self.status_message = format!("Smoke OK: {preview}");
                self.state.logs.info(format!("smoke test ok: {preview}"));
            }
            Err(err) => {
                self.status_message = format!("Smoke failed: {err:#}");
                self.state.logs.error(format!("smoke test failed: {err:#}"));
            }
        }
        self.smoke_running = false;
    }

    fn switch_tab(&mut self, tab: Tab) {
        self.tab = tab;
        if tab == Tab::Cli {
            self.ensure_cli();
        }
    }

    async fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
            return;
        }

        // Global quit always available with Ctrl+Q (even on CLI tab).
        if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if self.state.is_running() {
                let _ = stop_server(&self.state).await;
            }
            self.should_quit = true;
            return;
        }

        // Tab navigation that works while CLI has focus.
        if matches!(key.code, KeyCode::F(1)) {
            self.switch_tab(Tab::Dashboard);
            return;
        }
        if matches!(key.code, KeyCode::F(2)) {
            self.switch_tab(Tab::Config);
            return;
        }
        if matches!(key.code, KeyCode::F(3)) {
            self.switch_tab(Tab::Cli);
            return;
        }
        if matches!(key.code, KeyCode::F(4)) {
            self.switch_tab(Tab::Logs);
            return;
        }
        if matches!(key.code, KeyCode::F(5)) {
            self.switch_tab(Tab::Usage);
            return;
        }
        if matches!(key.code, KeyCode::F(6)) {
            self.switch_tab(Tab::Help);
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Left | KeyCode::Right)
        {
            if key.code == KeyCode::Right {
                self.switch_tab(self.tab.next());
            } else {
                self.switch_tab(self.tab.prev());
            }
            return;
        }

        if self.editing {
            match key.code {
                KeyCode::Esc => {
                    self.editing = false;
                    self.status_message = "Edit cancelled".into();
                }
                KeyCode::Enter => {
                    if let Err(err) = self.apply_edit() {
                        self.status_message = format!("Save failed: {err:#}");
                    }
                }
                KeyCode::Backspace => {
                    self.edit_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.edit_buffer.push(c);
                }
                _ => {}
            }
            return;
        }

        // CLI tab: forward keys into the PTY (except nav handled above).
        if self.tab == Tab::Cli {
            // Shift+R restarts the embedded shell.
            if key.code == KeyCode::Char('R') && key.modifiers.contains(KeyModifiers::SHIFT) {
                self.cli = None;
                self.ensure_cli();
                return;
            }
            if let Some(cli) = self.cli.as_mut() {
                if cli.alive() {
                    cli.handle_key(key);
                } else if key.code == KeyCode::Char('r') {
                    self.cli = None;
                    self.ensure_cli();
                }
            } else {
                self.ensure_cli();
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.state.is_running() {
                    let _ = stop_server(&self.state).await;
                }
                self.should_quit = true;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.state.is_running() {
                    let _ = stop_server(&self.state).await;
                }
                self.should_quit = true;
            }
            KeyCode::Tab | KeyCode::Right => self.switch_tab(self.tab.next()),
            KeyCode::BackTab | KeyCode::Left => self.switch_tab(self.tab.prev()),
            KeyCode::Char('1') => self.switch_tab(Tab::Dashboard),
            KeyCode::Char('2') => self.switch_tab(Tab::Config),
            KeyCode::Char('3') => self.switch_tab(Tab::Cli),
            KeyCode::Char('4') => self.switch_tab(Tab::Logs),
            KeyCode::Char('5') => self.switch_tab(Tab::Usage),
            KeyCode::Char('6') => self.switch_tab(Tab::Help),
            KeyCode::Char('s') if self.tab == Tab::Dashboard => {
                if self.state.is_running() {
                    match stop_server(&self.state).await {
                        Ok(()) => self.status_message = "Server stopped".into(),
                        Err(err) => self.status_message = format!("Stop failed: {err:#}"),
                    }
                } else {
                    match start_in_background(self.state.clone()).await {
                        Ok(()) => {
                            let cfg = self.state.config.read().clone();
                            self.status_message = crate::config::ready_banner(
                                &cfg,
                                self.state.agent_version.read().clone(),
                            );
                        }
                        Err(err) => self.status_message = format!("Start failed: {err:#}"),
                    }
                }
            }
            KeyCode::Char('r') if self.tab == Tab::Dashboard => {
                match self.state.backend.resolve_launch() {
                    Ok(launch) => match crate::cursor::probe_health_launch(&launch).await {
                        Ok(ver) => {
                            *self.state.agent_version.write() = Some(ver.clone());
                            self.status_message = format!("Agent OK: {ver} ({})", launch.display);
                            self.state.logs.info(format!("health check: {ver}"));
                        }
                        Err(err) => {
                            self.status_message = format!("Agent unhealthy: {err:#}");
                            self.state.logs.warn(format!("health check failed: {err:#}"));
                        }
                    },
                    Err(err) => {
                        self.status_message = format!("Agent missing: {err:#}");
                    }
                }
            }
            KeyCode::Char('t') if self.tab == Tab::Dashboard => {
                self.run_smoke_test().await;
            }
            KeyCode::Char('c') if self.tab == Tab::Logs => {
                self.state.logs.clear();
                self.status_message = "Logs cleared".into();
            }
            KeyCode::Char('x') if self.tab == Tab::Usage => {
                self.state.usage.reset();
                self.status_message = "Usage counters reset".into();
            }
            KeyCode::Up if self.tab == Tab::Config => {
                self.config_field = self.config_field.prev();
            }
            KeyCode::Down if self.tab == Tab::Config => {
                self.config_field = self.config_field.next();
            }
            KeyCode::Enter if self.tab == Tab::Config => self.begin_edit(),
            KeyCode::Char('e') if self.tab == Tab::Config => self.begin_edit(),
            KeyCode::Char('p') if self.tab == Tab::Config => self.cycle_profile(),
            KeyCode::Up if self.tab == Tab::Logs => {
                self.log_scroll = self.log_scroll.saturating_add(1);
            }
            KeyCode::Down if self.tab == Tab::Logs => {
                self.log_scroll = self.log_scroll.saturating_sub(1);
            }
            KeyCode::PageUp if self.tab == Tab::Logs => {
                self.log_scroll = self.log_scroll.saturating_add(10);
            }
            KeyCode::PageDown if self.tab == Tab::Logs => {
                self.log_scroll = self.log_scroll.saturating_sub(10);
            }
            KeyCode::Up if self.tab == Tab::Usage => {
                self.usage_scroll = self.usage_scroll.saturating_add(1);
            }
            KeyCode::Down if self.tab == Tab::Usage => {
                self.usage_scroll = self.usage_scroll.saturating_sub(1);
            }
            KeyCode::Up if self.tab == Tab::Help => {
                self.help_scroll = self.help_scroll.saturating_sub(1);
            }
            KeyCode::Down if self.tab == Tab::Help => {
                self.help_scroll = self.help_scroll.saturating_add(1);
            }
            KeyCode::PageUp if self.tab == Tab::Help => {
                self.help_scroll = self.help_scroll.saturating_sub(10);
            }
            KeyCode::PageDown if self.tab == Tab::Help => {
                self.help_scroll = self.help_scroll.saturating_add(10);
            }
            KeyCode::Home if self.tab == Tab::Help => {
                self.help_scroll = 0;
            }
            KeyCode::End if self.tab == Tab::Help => {
                let max = super::help::help_lines().len().saturating_sub(5) as u16;
                self.help_scroll = max;
            }
            KeyCode::Char('w') if self.tab == Tab::Config => {
                {
                    let mut cfg = self.state.config.write();
                    cfg.cursor.force = !cfg.cursor.force;
                    let path = self.state.config_path.read().clone();
                    let _ = cfg.save(&path);
                    self.status_message = format!(
                        "force={}",
                        if cfg.cursor.force { "on" } else { "off" }
                    );
                }
                self.state.reload_backend_from_config();
            }
            KeyCode::Char('j') if self.tab == Tab::Config => {
                {
                    let mut cfg = self.state.config.write();
                    cfg.cursor.json_mode = !cfg.cursor.json_mode;
                    let path = self.state.config_path.read().clone();
                    let _ = cfg.save(&path);
                    self.status_message = format!(
                        "json_mode={}",
                        if cfg.cursor.json_mode { "on" } else { "off" }
                    );
                }
                self.state.reload_backend_from_config();
            }
            KeyCode::Char('k') if self.tab == Tab::Config => {
                {
                    let mut cfg = self.state.config.write();
                    cfg.cursor.trust = !cfg.cursor.trust;
                    let path = self.state.config_path.read().clone();
                    let _ = cfg.save(&path);
                    self.status_message =
                        format!("trust={}", if cfg.cursor.trust { "on" } else { "off" });
                }
                self.state.reload_backend_from_config();
            }
            _ => {}
        }
    }
}

pub async fn run_tui(state: AppState, autostart: bool) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = TuiApp::new(state);
    if autostart {
        match start_in_background(app.state.clone()).await {
            Ok(()) => {
                let cfg = app.state.config.read().clone();
                app.status_message =
                    crate::config::ready_banner(&cfg, app.state.agent_version.read().clone());
            }
            Err(err) => app.status_message = format!("Autostart failed: {err:#}"),
        }
    }

    let mut events = EventStream::new();
    let mut ticker = tokio::time::interval(Duration::from_millis(50));

    let result = loop {
        terminal.draw(|frame| {
            let area = frame.area();
            // Inner CLI size ≈ content area minus chrome.
            let cli_cols = area.width.saturating_sub(4).max(40);
            let cli_rows = area.height.saturating_sub(8).max(10);
            if app.tab == Tab::Cli {
                if app.cli_cols != cli_cols || app.cli_rows != cli_rows {
                    app.cli_cols = cli_cols;
                    app.cli_rows = cli_rows;
                    if let Some(cli) = app.cli.as_mut() {
                        cli.resize(cli_cols, cli_rows);
                    }
                }
            }
            super::ui::draw(frame, &app);
        })?;

        tokio::select! {
            maybe = events.next() => {
                match maybe {
                    Some(Ok(Event::Key(key))) => {
                        app.handle_key(key).await;
                        if app.should_quit {
                            break Ok(());
                        }
                    }
                    Some(Ok(Event::Resize(_, _))) => {}
                    Some(Err(err)) => break Err(err.into()),
                    None => break Ok(()),
                    _ => {}
                }
            }
            _ = ticker.tick() => {
                app.tick_cli();
            }
        }
    };

    if app.state.is_running() {
        let _ = stop_server(&app.state).await;
    }
    app.cli = None;
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    result
}
