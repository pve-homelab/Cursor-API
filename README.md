# Cursor-API

CLI command: `cursor-api` (lowercase). Display name: **Cursor-API**.

Local **OpenAI-compatible `/v1` API** backed by the **Cursor Agent CLI**, with a **Ratatui TUI** to start/stop the service, edit config, watch logs, and track estimated token usage.

Works on **Windows**, **macOS**, and **Linux**.

```
Your app  →  http://127.0.0.1:8787/v1/chat/completions  →  agent -p (Cursor CLI)
```

---

## Getting started

**Important:** do **not** run interactive `agent` chat as your server. Login once, then start **Cursor-API** (`cursor-api`) — that is the control panel. API calls spawn short headless `agent -p` jobs in the background; you stay in the TUI.

### Quick start

The same three commands build and start the project on every operating system:

```text
git clone https://github.com/pve-homelab/Cursor-API.git
cd Cursor-API
cargo run --release
```

The first build downloads Rust dependencies and may take a few minutes. When the TUI opens, the API is available at `http://127.0.0.1:8787/v1`. Keep that terminal open while using the API.

Before running those commands, complete the one-time setup for your operating system below.

### Shared prerequisites (all platforms)

1. Install **Rust**: https://rustup.rs/
2. Install the **Cursor CLI** (`agent`): https://cursor.com/docs/cli/overview
3. Log in **once** (this is interactive — finish it, then close that prompt):

```bash
agent login
agent --version
```

---

### Windows (PowerShell)

Use **Windows Terminal** or PowerShell 7+ if you can.

**1. Install Rust** (if needed) — open https://rustup.rs/ and run the Windows installer, then reopen the terminal.

**2. Install + log in to Cursor CLI** (one-time):

```powershell
irm 'https://cursor.com/install?win32=true' | iex
agent login
agent --version
```

**3. Build and start Cursor-API (TUI + API)**

```powershell
git clone https://github.com/pve-homelab/Cursor-API.git
cd Cursor-API
cargo run --release
```

That opens the **TUI** and **starts the HTTP API automatically**. Leave this window open.

You should see a ready line like:

```text
Ready · http://127.0.0.1:8787/v1 · /health → 200 · model=auto · mode=ask · …
```

Use `model=auto` (or whatever the banner shows) in your client.

**TUI keys:** `1` Dashboard · `2` Config · `q` quit · `s` stop/start API · `t` smoke test

*(Headless only, no TUI: `.\target\release\cursor-api.exe serve` — not recommended for first-time use.)*

**4. Health check + test prompt** (new PowerShell window)

```powershell
# Health (note default_model)
Invoke-RestMethod http://127.0.0.1:8787/health | Format-List status, v1_url, default_model, mode, healthy

# Send a prompt and print the reply
$body = @{
  model = "auto"
  messages = @(@{ role = "user"; content = "Say hi in one sentence." })
} | ConvertTo-Json -Depth 5

$res = Invoke-RestMethod http://127.0.0.1:8787/v1/chat/completions `
  -Method Post -ContentType "application/json" -Body $body

$res.choices[0].message.content
```

**Stop:** in the TUI press `s` (stop server) then `q` (quit).

---

### macOS (Terminal / zsh)

**1. Install Rust** (if needed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustc --version
```

**2. Install + log in to Cursor CLI** (one-time):

```bash
curl https://cursor.com/install -fsS | bash
export PATH="$HOME/.local/bin:$PATH"   # add to ~/.zshrc if needed
agent login
agent --version
```

**3. Build and start Cursor-API (TUI + API)**

```bash
git clone https://github.com/pve-homelab/Cursor-API.git
cd Cursor-API
cargo run --release
```

That opens the **TUI** and **starts the HTTP API automatically**. Leave this terminal open.

You should see a ready line like:

```text
Ready · http://127.0.0.1:8787/v1 · /health → 200 · model=auto · mode=ask · …
```

Use `model=auto` (or whatever the banner shows) in your client.

**TUI keys:** `1` Dashboard · `2` Config · `q` quit · `s` stop/start API · `t` smoke test

*(Headless only, no TUI: `./target/release/cursor-api serve` — not recommended for first-time use.)*

**4. Health check + test prompt** (new terminal tab)

```bash
curl -s http://127.0.0.1:8787/health | python3 -m json.tool
curl -s http://127.0.0.1:8787/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"auto","messages":[{"role":"user","content":"Say hi in one sentence."}]}' \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['choices'][0]['message']['content'])"
```

**Stop:** in the TUI press `s` (stop server) then `q` (quit).

---

### Linux (bash)

**1. Install Rust** (if needed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustc --version
```

Also install a C toolchain if cargo complains (Debian/Ubuntu example):

```bash
sudo apt update && sudo apt install -y build-essential pkg-config
```

**2. Install + log in to Cursor CLI** (one-time):

```bash
curl https://cursor.com/install -fsS | bash
export PATH="$HOME/.local/bin:$PATH"   # add to ~/.bashrc if needed
agent login
agent --version
```

**3. Build and start Cursor-API (TUI + API)**

```bash
git clone https://github.com/pve-homelab/Cursor-API.git
cd Cursor-API
cargo run --release
```

That opens the **TUI** and **starts the HTTP API automatically**. Leave this terminal open.

You should see a ready line like:

```text
Ready · http://127.0.0.1:8787/v1 · /health → 200 · model=auto · mode=ask · …
```

Use `model=auto` (or whatever the banner shows) in your client.

**TUI keys:** `1` Dashboard · `2` Config · `q` quit · `s` stop/start API · `t` smoke test

*(Headless only, no TUI: `./target/release/cursor-api serve` — not recommended for first-time use.)*

**4. Health check + test prompt** (new terminal)

```bash
curl -s http://127.0.0.1:8787/health | python3 -m json.tool
curl -s http://127.0.0.1:8787/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"auto","messages":[{"role":"user","content":"Say hi in one sentence."}]}' \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['choices'][0]['message']['content'])"
```

**Stop:** in the TUI press `s` (stop server) then `q` (quit).

---

### If something fails

| Symptom | Fix |
|---------|-----|
| Stuck in `agent` / login UI | That is **not** the bridge. Finish or Ctrl+C, then run `cursor-api` (the TUI) |
| `agent` not found | Install Cursor CLI, ensure it’s on `PATH`, reopen the terminal |
| Auth / login errors | Run `agent login` once in a normal shell (or set Cursor API key in TUI Config) |
| Can’t connect to `8787` | Make sure the TUI is still open and shows RUNNING (press `s` if stopped) |
| `failed to spawn cursor agent` | Run `cursor-api doctor`. Bridge prefers bundled Node: Windows `%LOCALAPPDATA%\cursor-agent\versions\*\node.exe`, macOS/Linux `~/.local/share/cursor-agent/versions/*/node` |
| Slow first reply | Normal — first Cursor CLI run can take a bit |
| TUI without auto-start | `cursor-api tui --no-autostart` then press `s` |

---

## Commands

| Command | Description |
|---------|-------------|
| `cursor-api` | **TUI + start HTTP API** (default — use this) |
| `cursor-api tui --no-autostart` | TUI only; press `s` to start API |
| `cursor-api serve` | Headless HTTP only (no TUI) |
| `cursor-api doctor` | Locate/probe the `agent` binary |
| `cursor-api doctor --full` | Probe + live smoke completion |
| `cursor-api config-path` | Print `config.toml` location |

Install onto your cargo bin directory (optional):

```bash
cargo install --path .
cursor-api
```

## TUI

| Tab | Keys |
|-----|------|
| Dashboard | `s` start/stop · `r` health-check · `t` smoke test |
| Config | `↑/↓` select · `Enter` edit · `p` cycle profile · `j` json mode · `w` force · `k` trust |
| Logs | `↑/↓` scroll · `c` clear |
| Usage | token estimates + recent requests · `x` reset |
| Help | quick client notes + compatibility |

`Tab` / `1-5` switch tabs · `q` quit

## More API examples

Default base URL: `http://127.0.0.1:8787/v1`  
Default model: whatever `/health` reports as `default_model` (usually `auto`)

```bash
# Streaming SSE
curl http://127.0.0.1:8787/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"auto","stream":true,"messages":[{"role":"user","content":"Say hi."}]}'

# List models
curl http://127.0.0.1:8787/v1/models
```

### OpenAI SDK example

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://127.0.0.1:8787/v1",
    api_key="local",  # set Bridge API key in TUI if you enabled auth
)

print(client.chat.completions.create(
    model="auto",
    messages=[{"role": "user", "content": "Hello from my app"}],
).choices[0].message.content)
```

## Configuration

Created on first run. Find it with:

```bash
cursor-api config-path
```

Typical locations:

| Platform | Config file |
|----------|-------------|
| Linux | `~/.config/cursor-api/config.toml` |
| macOS | `~/Library/Application Support/cursor-api/config.toml` |
| Windows | `%APPDATA%\cursor-api\config.toml` |

Important fields:

| Field | Default | Notes |
|-------|---------|-------|
| `server.host` / `port` | `127.0.0.1` / `8787` | Bind locally |
| `server.request_timeout_secs` | `600` | Per-request agent timeout |
| `server.max_concurrency` | `2` | Max parallel agent processes |
| `server.reject_when_busy` | `true` | Return HTTP 429 when slots are full (after optional queue wait) |
| `server.queue_wait_secs` | `0` (`1800` in `long_running`) | Wait for a free slot before 429 |
| `cursor.profile` | `chat` | Preset: `chat`, `json_api`, `long_running` |
| `cursor.mode` | `ask` | Chat-safe; use `agent` + `force` only if you want file edits |
| `cursor.json_mode` | `false` | Append JSON instruction + normalize responses |
| `cursor.message_flatten_mode` | `flat` | `flat`, `fold_system`, or `system_last` |
| `cursor.default_model` | `auto` | Passed to `--model` — also shown on ready banner + `/health` |
| `cursor.workspace` | empty | Optional `--workspace` |
| `cursor.binary` | empty | Auto-detect `agent` / `cursor-agent` on `PATH` |
| `auth.api_key` | auto-generated | If set, require `Authorization: Bearer …` on `/v1` |

### Environment overrides

Prefer **`CURSOR_API_*`** so a leftover `BRIDGE_PORT` from a sibling bridge (e.g. Kiro-API) cannot steal this process’s port. Legacy `BRIDGE_*` still works as a fallback and logs a warning.

| Variable | Effect |
|----------|--------|
| `CURSOR_API_HOST` / `BRIDGE_HOST` | Override bind host |
| `CURSOR_API_PORT` / `BRIDGE_PORT` | Override port |
| `CURSOR_API_KEY` / `BRIDGE_API_KEY` | Set bridge auth key |
| `CURSOR_API_TIMEOUT_SECS` / `BRIDGE_TIMEOUT_SECS` | Override request timeout |
| `CURSOR_API_JSON_MODE` / `BRIDGE_JSON_MODE` | `true`/`1` enables JSON mode |
| `CURSOR_API_DEFAULT_MODEL` / `BRIDGE_DEFAULT_MODEL` | Override default model |
| `CURSOR_WORKSPACE` | Override workspace path |

Startup logs and `/health` report `bind_source` (`config` vs env name) plus `available_permits` so you can tell “legitimately busy” from a stuck slot.

## How it works

- Non-stream: `agent -p --output-format json …`
- Stream: `agent -p --output-format stream-json --stream-partial-output …`
- Responses are mapped to OpenAI `chat.completion` / SSE chunks
- `response_format: { "type": "json_object" }` triggers JSON extraction (strips markdown fences)
- Streaming sends a final SSE chunk with `usage` before `[DONE]`
- Send `X-Request-ID` to correlate requests in logs and response headers
- HTTP **429** when `max_concurrency` slots are full (`reject_when_busy=true`)
- Client disconnect cancels the underlying agent process
- Token totals are **estimates** (`chars/4`) unless Cursor exposes billing usage later

### OpenAI compatibility matrix

| Feature | Supported |
|---------|-----------|
| `/v1/chat/completions` sync | Yes |
| `/v1/chat/completions` stream (SSE) | Yes |
| `/v1/models` | Yes |
| `response_format` JSON | Yes |
| `X-Request-ID` | Yes |
| Final stream `usage` chunk | Yes |
| HTTP 429 when busy | Yes |
| `temperature` / `max_tokens` | Ignored (CLI has no equivalent) |
| Tool / function calling | No |

## Cross-platform notes

- The HTTP server and TUI use portable crates (`tokio`, `axum`, `crossterm`, `ratatui`).
- The Cursor CLI is resolved the same way on every OS: bundled Node + `index.js` first (Windows `%LOCALAPPDATA%\cursor-agent`, macOS/Linux `~/.local/share/cursor-agent`), then `PATH`, then common install locations.
- Prefer a UTF-8 terminal. On Windows, Windows Terminal or a modern PowerShell host works best for the TUI.
- Default command is always **TUI + autostart API** on Windows, macOS, and Linux. Use `serve` only for systemd/launchd/NSSM supervisors.

## Safety

- Defaults to **ask** mode (no writes)
- Keep `force=false` unless you intentionally want the agent to edit your workspace
- Prefer binding `127.0.0.1` and setting a bridge API key for anything beyond solo local use

## License

MIT
