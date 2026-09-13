use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub cursor: CursorConfig,
    pub auth: AuthConfig,
    pub logging: LoggingConfig,
    /// Where host/port came from after env overrides (not persisted).
    #[serde(skip)]
    pub bind_source: BindSource,
}

/// Effective bind override provenance for startup logs / /health.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindSource {
    pub host: &'static str,
    pub port: &'static str,
}

impl Default for BindSource {
    fn default() -> Self {
        Self {
            host: "config",
            port: "config",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    /// Max concurrent Cursor CLI processes.
    pub max_concurrency: usize,
    /// Per-request timeout in seconds.
    pub request_timeout_secs: u64,
    /// When true, return HTTP 429 immediately if max_concurrency slots are full.
    pub reject_when_busy: bool,
    /// Seconds to wait for a concurrency slot before returning 429 (0 = no wait).
    pub queue_wait_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CursorConfig {
    /// Path to the `agent` / `cursor-agent` binary. Empty = auto-detect.
    pub binary: String,
    /// Default model id passed to `--model`.
    pub default_model: String,
    /// Workspace directory for agent runs. Empty = process cwd.
    pub workspace: String,
    /// ask | agent | plan
    pub mode: String,
    /// Pass `--trust` for headless runs.
    pub trust: bool,
    /// Pass `--force` (allows writes/shell). Keep false for chat-only proxies.
    pub force: bool,
    /// Extra CLI args appended to every invocation.
    pub extra_args: Vec<String>,
    /// API key forwarded to Cursor CLI (`CURSOR_API_KEY`).
    pub cursor_api_key: String,
    /// Append JSON-only instruction and normalize JSON in responses.
    pub json_mode: bool,
    /// flat | fold_system | system_last
    pub message_flatten_mode: String,
    /// Prepended to every prompt.
    pub prompt_prefix: String,
    /// Appended to every prompt.
    pub prompt_suffix: String,
    /// Preset profile: chat | json_api | long_running (applied on load/save if set).
    pub profile: String,
    /// Advertised context budget (tokens). Soft warn when prompts exceed this.
    pub max_context_tokens: u32,
    /// When true, truncate prompts that exceed max_context_tokens (default false).
    pub truncate_over_context: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    /// If set, require `Authorization: Bearer <key>` on /v1 routes.
    pub api_key: String,
    /// Bind advice shown in TUI; server honors host.
    pub require_auth: bool,
    /// Generate a random bridge API key when creating a new config file.
    pub generate_api_key_on_first_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: String,
    /// Max in-memory log lines for the TUI.
    pub ring_capacity: usize,
    /// Optional log file path.
    pub file: String,
    /// Redact bearer tokens and API keys in log messages.
    pub redact_secrets: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            cursor: CursorConfig::default(),
            auth: AuthConfig::default(),
            logging: LoggingConfig::default(),
            bind_source: BindSource::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 8787,
            max_concurrency: 2,
            request_timeout_secs: 600,
            reject_when_busy: true,
            queue_wait_secs: 0,
        }
    }
}

impl Default for CursorConfig {
    fn default() -> Self {
        Self {
            binary: String::new(),
            default_model: "auto".into(),
            workspace: String::new(),
            mode: "ask".into(),
            trust: true,
            force: false,
            extra_args: Vec::new(),
            cursor_api_key: String::new(),
            json_mode: false,
            message_flatten_mode: "flat".into(),
            prompt_prefix: String::new(),
            prompt_suffix: String::new(),
            profile: "chat".into(),
            max_context_tokens: 128_000,
            truncate_over_context: false,
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            require_auth: false,
            generate_api_key_on_first_run: true,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".into(),
            ring_capacity: 2000,
            file: String::new(),
            redact_secrets: true,
        }
    }
}

impl Config {
    pub fn config_dir() -> Result<PathBuf> {
        let base = dirs::config_dir().context("could not resolve config directory")?;
        Ok(base.join("cursor-api"))
    }

    /// Prefer new config dir; migrate from legacy `cursor-v1-bridge` on first run if needed.
    pub fn default_path() -> Result<PathBuf> {
        let path = Self::config_dir()?.join("config.toml");
        if !path.exists() {
            if let Some(legacy) = legacy_config_path() {
                if legacy.exists() {
                    if let Some(parent) = path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let _ = fs::copy(&legacy, &path);
                }
            }
        }
        Ok(path)
    }

    pub fn load_or_create() -> Result<(Self, PathBuf)> {
        let path = Self::default_path()?;
        if path.exists() {
            let mut cfg = Self::load(&path)?;
            cfg.apply_env_overrides();
            Ok((cfg, path))
        } else {
            let mut cfg = Self::default();
            if cfg.auth.generate_api_key_on_first_run {
                cfg.auth.api_key = generate_bridge_api_key();
                cfg.auth.require_auth = true;
            }
            cfg.apply_profile();
            cfg.apply_env_overrides();
            cfg.save(&path)?;
            Ok((cfg, path))
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config {}", path.display()))?;
        let mut cfg: Config = toml::from_str(&raw)
            .with_context(|| format!("failed to parse config {}", path.display()))?;
        cfg.apply_profile();
        Ok(cfg)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw = toml::to_string_pretty(self).context("serialize config")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    pub fn listen_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    pub fn base_url(&self) -> String {
        format!("http://{}:{}", self.server.host, self.server.port)
    }

    pub fn v1_url(&self) -> String {
        format!("{}/v1", self.base_url())
    }

    /// Apply named profile presets (chat, json_api, long_running).
    pub fn apply_profile(&mut self) {
        match self.cursor.profile.trim().to_lowercase().as_str() {
            "json_api" | "json-api" | "json" => {
                self.cursor.mode = "ask".into();
                self.cursor.json_mode = true;
                self.cursor.message_flatten_mode = "fold_system".into();
                if self.cursor.prompt_suffix.is_empty() {
                    self.cursor.prompt_suffix =
                        "Respond with valid JSON only. No markdown fences.".into();
                }
                self.cursor.force = false;
            }
            "long_running" | "long-running" | "long" => {
                self.server.request_timeout_secs = 900;
                self.server.max_concurrency = 1;
                // Queue instead of hard-429 so batched clients can wait for the single slot
                // instead of aborting immediately.
                self.server.reject_when_busy = true;
                self.server.queue_wait_secs = 1800;
                self.cursor.mode = "ask".into();
                self.cursor.force = false;
            }
            _ => {
                // chat — keep defaults unless user customized
            }
        }
    }

    pub fn set_profile(&mut self, profile: &str) {
        self.cursor.profile = profile.into();
        self.apply_profile();
    }

    pub fn apply_env_overrides(&mut self) {
        self.bind_source = BindSource::default();

        // Prefer product-specific vars so a leftover BRIDGE_PORT from another
        // bridge process cannot silently steal Cursor-API's configured port.
        if let Some(host) = first_nonempty_env(&["CURSOR_API_HOST", "BRIDGE_HOST"]) {
            let source = if std::env::var("CURSOR_API_HOST")
                .ok()
                .filter(|v| !v.is_empty())
                .is_some()
            {
                "CURSOR_API_HOST"
            } else {
                "BRIDGE_HOST"
            };
            if source == "BRIDGE_HOST" {
                tracing::warn!(
                    "BRIDGE_HOST is set — prefer CURSOR_API_HOST (BRIDGE_* is a shared legacy name)"
                );
            }
            self.server.host = host;
            self.bind_source.host = source;
        }

        if let Some(port_raw) = first_nonempty_env(&["CURSOR_API_PORT", "BRIDGE_PORT"]) {
            let source = if std::env::var("CURSOR_API_PORT")
                .ok()
                .filter(|v| !v.is_empty())
                .is_some()
            {
                "CURSOR_API_PORT"
            } else {
                "BRIDGE_PORT"
            };
            if let Ok(p) = port_raw.parse() {
                if source == "BRIDGE_PORT" {
                    tracing::warn!(
                        "BRIDGE_PORT={port_raw} overrides config port {} — prefer CURSOR_API_PORT (BRIDGE_* is a shared legacy name)",
                        self.server.port
                    );
                }
                self.server.port = p;
                self.bind_source.port = source;
            }
        }

        if let Some(key) = first_nonempty_env(&["CURSOR_API_KEY", "BRIDGE_API_KEY"]) {
            self.auth.api_key = key;
            self.auth.require_auth = true;
        }

        if let Ok(ws) = std::env::var("CURSOR_WORKSPACE") {
            self.cursor.workspace = ws;
        }

        if let Some(model) =
            first_nonempty_env(&["CURSOR_API_DEFAULT_MODEL", "BRIDGE_DEFAULT_MODEL"])
        {
            self.cursor.default_model = model;
        }

        if let Some(timeout) =
            first_nonempty_env(&["CURSOR_API_TIMEOUT_SECS", "BRIDGE_TIMEOUT_SECS"])
        {
            if let Ok(t) = timeout.parse() {
                self.server.request_timeout_secs = t;
            }
        }

        if let Some(v) = first_nonempty_env(&["CURSOR_API_JSON_MODE", "BRIDGE_JSON_MODE"]) {
            self.cursor.json_mode = matches!(v.to_lowercase().as_str(), "1" | "true" | "yes");
        }

        if let Some(v) =
            first_nonempty_env(&["CURSOR_API_MAX_CONTEXT_TOKENS", "BRIDGE_MAX_CONTEXT_TOKENS"])
        {
            if let Ok(n) = v.parse::<u32>() {
                if n > 0 {
                    self.cursor.max_context_tokens = n;
                }
            }
        }

        if let Some(v) = first_nonempty_env(&[
            "CURSOR_API_TRUNCATE_OVER_CONTEXT",
            "BRIDGE_TRUNCATE_OVER_CONTEXT",
        ]) {
            self.cursor.truncate_over_context =
                matches!(v.to_lowercase().as_str(), "1" | "true" | "yes");
        }
    }

    pub fn bind_source_summary(&self) -> String {
        format!(
            "host={} ({}) port={} ({})",
            self.server.host,
            self.bind_source.host,
            self.server.port,
            self.bind_source.port
        )
    }
}

fn first_nonempty_env(keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Ok(v) = std::env::var(key) {
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

/// Human-readable ready banner for logs / stdout / TUI status.
pub fn ready_banner(cfg: &Config, agent_version: Option<String>) -> String {
    let agent = agent_version.unwrap_or_else(|| "unknown".into());
    let auth = if cfg.auth.require_auth && !cfg.auth.api_key.is_empty() {
        "Bearer auth ON"
    } else {
        "auth off"
    };
    format!(
        "Cursor-API ready · {} · /health → 200 · model={} · mode={} · profile={} · agent={} · {}",
        cfg.v1_url(),
        cfg.cursor.default_model,
        cfg.cursor.mode,
        cfg.cursor.profile,
        agent,
        auth,
    )
}

fn legacy_config_path() -> Option<PathBuf> {
    let base = dirs::config_dir()?;
    Some(base.join("cursor-v1-bridge").join("config.toml"))
}

pub fn generate_bridge_api_key() -> String {
    format!("bridge-{}", uuid::Uuid::new_v4())
}

pub fn redact_secrets(text: &str, enabled: bool) -> String {
    if !enabled {
        return text.to_string();
    }
    let mut out = text.to_string();
    for prefix in ["Bearer ", "bearer ", "bridge-", "sk-"] {
        if let Some(idx) = out.find(prefix) {
            let start = idx + prefix.len();
            let end = out[start..]
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                .map(|i| start + i)
                .unwrap_or(out.len());
            if end > start {
                out.replace_range(start..end, "***");
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn cursor_api_port_wins_over_bridge_port() {
        let _guard = env_lock().lock().unwrap();
        std::env::set_var("BRIDGE_PORT", "8788");
        std::env::set_var("CURSOR_API_PORT", "8787");
        let mut cfg = Config::default();
        cfg.server.port = 9999;
        cfg.apply_env_overrides();
        assert_eq!(cfg.server.port, 8787);
        assert_eq!(cfg.bind_source.port, "CURSOR_API_PORT");
        std::env::remove_var("BRIDGE_PORT");
        std::env::remove_var("CURSOR_API_PORT");
    }

    #[test]
    fn long_running_profile_queues_instead_of_instant_429() {
        let mut cfg = Config::default();
        cfg.set_profile("long_running");
        assert_eq!(cfg.server.max_concurrency, 1);
        assert!(cfg.server.queue_wait_secs > 0);
        assert!(cfg.server.reject_when_busy);
    }
}
