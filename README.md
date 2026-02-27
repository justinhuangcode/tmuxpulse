# TmuxPulse

**English** | [中文](./README_CN.md)

[![CI](https://github.com/justinhuangcode/tmuxpulse/actions/workflows/ci.yml/badge.svg)](https://github.com/justinhuangcode/tmuxpulse/actions/workflows/ci.yml)
[![Release](https://github.com/justinhuangcode/tmuxpulse/actions/workflows/release.yml/badge.svg)](https://github.com/justinhuangcode/tmuxpulse/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/tmuxpulse?style=flat-square)](https://crates.io/crates/tmuxpulse)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey?style=flat-square)](https://github.com/justinhuangcode/tmuxpulse)
[![GitHub Stars](https://img.shields.io/github/stars/justinhuangcode/tmuxpulse?style=flat-square&logo=github)](https://github.com/justinhuangcode/tmuxpulse/stargazers)
[![Last Commit](https://img.shields.io/github/last-commit/justinhuangcode/tmuxpulse?style=flat-square)](https://github.com/justinhuangcode/tmuxpulse/commits/main)
[![Issues](https://img.shields.io/github/issues/justinhuangcode/tmuxpulse?style=flat-square)](https://github.com/justinhuangcode/tmuxpulse/issues)

A real-time, event-driven tmux TUI for session monitoring, pane search, and stale cleanup. 💓

See all your sessions at a glance, navigate with keyboard or mouse, search across panes, and manage stale sessions -- all from a single terminal window.

## tmux Terminology

tmux organizes work in a three-level hierarchy. Understanding this makes the rest of the docs much clearer:

```
Session (independent workspace, e.g. "backend", "frontend")
  └── Window (tab within a session, switched with Ctrl-b n)
        └── Pane (split within a window, e.g. left: server, right: logs)
```

| Concept | Analogy | Example |
|---|---|---|
| **Session** | A virtual desktop | `tmux new -s backend` creates a session named "backend" |
| **Window** | A tab in that desktop | One window for `vim`, another for `git` |
| **Pane** | A split within a tab | Left pane runs the server, right pane tails logs |

A developer might have 10 sessions, each with 2-3 windows, each with 1-2 panes. That's 20-60 terminal viewports -- and `tmux ls` only shows the session names.

## Why tmuxpulse?

tmux power users run dozens of sessions simultaneously. To check what's happening across them, you have to manually cycle through `tmux ls` (list sessions) → `tmux attach -t <name>` (enter a session) → look around → `Ctrl-b d` (detach) → repeat for the next session. Every switch breaks your focus.

tmuxpulse gives you **a single dashboard showing all sessions, windows, and panes in real time** -- without leaving your current terminal.

Existing tools don't fit this workflow:

| | tmuxpulse | tmuxwatch | htop/btop | tmux built-in |
|---|---|---|---|---|
| Designed for tmux monitoring | Yes | Yes | No (process monitor) | No (session manager) |
| Architecture | Event-driven via `tmux -C` | Polling (1s) | Polling | N/A |
| Language / runtime | Rust (single binary) | Go (single binary) | C/C++ | N/A |
| Config file | TOML + CLI overrides | None | Yes | tmux.conf (no TUI config) |
| Theme engine | 4 built-in themes (20 color slots) | Hardcoded 256-color | Yes | N/A |
| Plugin system | Script-based (TOML manifest + JSON protocol) | None | None | N/A |
| Daemon RPC | JSON-RPC 2.0 over Unix socket | None | None | N/A |
| Workspace snapshots | Save / restore layouts as JSON | Planned, not implemented | N/A | N/A |
| Multiplexer abstraction | Trait-based (future: Zellij) | tmux only | N/A | tmux only |
| Incremental capture | FNV-1a content hash diffing | Full snapshot each poll | N/A | N/A |
| Search | Fuzzy across sessions/windows/panes | Exact match | Process filter | N/A |
| Binary size | ~2MB (stripped, LTO) | ~15MB | Varies | N/A |

**The typical developer workflow with tmuxpulse:**

```
Developer has 10+ tmux sessions running
        |
tmuxpulse renders all sessions in an adaptive grid
        |
Developer sees pane output live, spots errors instantly
        |
Developer presses Enter to focus a session, scrolls output
        |
Developer presses X to kill stale sessions, / to search
        |
Developer presses q to return to work
```

No manual session cycling. No lost context. Just a persistent overview that reflects your tmux state in real time.

## Features

- **Event-driven monitoring** -- tmux control mode (`tmux -C`) provides a structured event stream instead of polling; events arrive in <10ms
- **Adaptive grid layout** -- Session cards auto-arrange based on terminal size with configurable min width/height
- **Live pane capture** -- Real-time output from active panes with smart scrolling (auto-scroll when at bottom, preserve manual scroll position)
- **Activity pulse** -- Border animation highlights panes with new output (1.5s pulse duration)
- **Stale detection** -- Sessions with all dead panes or >1h idle are marked stale and can be killed in bulk
- **Keyboard + mouse navigation** -- Arrow keys for grid movement, Enter to focus, mouse click on cards, scroll wheel for output
- **Command palette** -- `Ctrl+P` opens a keyboard-navigable action menu (refresh, show hidden, expand all, kill stale)
- **Tab system** -- Overview grid tab + per-session detail tabs; `Shift+Left/Right` to cycle, `Ctrl+W` to close
- **Search / filter** -- `/` opens live search across session names, window names, and pane commands
- **Workspace snapshots** -- `tmuxpulse workspace save dev-setup` saves your tmux topology as JSON for later restore
- **Plugin system** -- Extend tmuxpulse with external scripts via TOML manifests and JSON stdin/stdout protocol; lifecycle hooks for init, on_snapshot, on_event, and shutdown
- **Daemon RPC** -- `tmuxpulse daemon start` launches a JSON-RPC 2.0 server over Unix socket for AI agents, scripts, and external tools
- **TOML configuration** -- Every visual and behavioral aspect configurable: themes, keybindings, capture depth, stale threshold, border style
- **4 built-in themes** -- Default, Catppuccin Mocha, Dracula, Nord -- or define your own in TOML
- **Budget-based capture** -- Max 6 pane captures per tick with priority scheduling: focused > cursor > round-robin
- **Incremental diffing** -- FNV-1a content hashing skips re-render when pane output hasn't changed
- **JSON output** -- `--dump --json` for machine-readable snapshot output, suitable for scripting and AI agents
- **Cross-platform** -- macOS and Linux (tmux is required; Windows via WSL)

## Installation

### Pre-built binaries (recommended)

Download the latest binary for your platform from [GitHub Releases](https://github.com/justinhuangcode/tmuxpulse/releases):

| Platform | Archive |
| --- | --- |
| Linux x86_64 | `tmuxpulse-linux-amd64.tar.gz` |
| Linux ARM64 | `tmuxpulse-linux-arm64.tar.gz` |
| macOS Intel | `tmuxpulse-macos-amd64.tar.gz` |
| macOS Apple Silicon | `tmuxpulse-macos-arm64.tar.gz` |

Extract the archive and place the binary in your `$PATH`.

### Homebrew (macOS / Linux)

```bash
brew tap justinhuangcode/tap
brew install tmuxpulse
```

### Via Cargo (crates.io)

```bash
cargo install tmuxpulse
```

### From source

```bash
git clone https://github.com/justinhuangcode/tmuxpulse.git
cd tmuxpulse
cargo install --path .
```

**Requirements:** Rust 1.88+ and tmux 3.1+. tmux must be running (`tmux new -s mysession`) before launching tmuxpulse.

## Quick Start

```bash
# Make sure tmux is running
tmux new -s dev -d
tmux new -s build -d

# Launch the TUI
tmuxpulse

# Dump session info as text
tmuxpulse --dump

# Dump as JSON (for scripting / AI agents)
tmuxpulse --dump --json

# Use a specific theme
tmuxpulse --theme catppuccin-mocha

# Custom poll interval
tmuxpulse --interval 500ms

# Generate default config file
tmuxpulse config init

# Show current effective config
tmuxpulse config show

# Start daemon for AI agent access
tmuxpulse daemon start

# Query daemon from scripts
tmuxpulse daemon call pulse.sessions
tmuxpulse daemon call pulse.capture '{"pane_id": "%1", "lines": 50}'
```

## Commands

| Command | Description |
| --- | --- |
| *(default)* | Launch the TUI monitor |
| `config init` | Generate default config at `~/.config/tmuxpulse/config.toml` |
| `config show` | Print current effective configuration |
| `workspace save <name>` | Save current tmux layout as a named snapshot |
| `workspace restore <name>` | Restore a saved workspace |
| `workspace list` | List all saved workspaces |
| `plugin list` | List installed plugins |
| `plugin install <path>` | Install a plugin from a local directory |
| `daemon start` | Launch daemon RPC server in foreground |
| `daemon status` | Check if daemon is running and show version/uptime |
| `daemon stop` | Stop a running daemon |
| `daemon call <method> [params]` | Send an RPC call to the daemon (JSON params) |

## Global Flags

| Flag | Default | Description |
| --- | --- | --- |
| `--interval <duration>` | `1s` | Poll interval fallback (e.g. `1s`, `500ms`, `2m`) |
| `--tmux <path>` | auto-detect | Path to tmux binary |
| `--dump` | false | Print snapshot and exit (text format) |
| `--json` | false | Machine-readable JSON output (combine with `--dump`) |
| `-c, --config <path>` | auto | Configuration file path |
| `--theme <name>` | config value | Override theme: `default`, `catppuccin-mocha`, `dracula`, `nord` |

## Daemon Start Flags

| Flag | Default | Description |
| --- | --- | --- |
| `--socket <path>` | `$XDG_RUNTIME_DIR/tmuxpulse.sock` | Custom Unix socket path |

## Keyboard Shortcuts

### Grid View (Overview)

| Key | Action |
| --- | --- |
| `Arrow keys` | Navigate between session cards |
| `Enter` | Focus the selected session (full-height output) |
| `/` | Open live search filter |
| `Ctrl+P` | Open command palette |
| `z` | Collapse / expand selected session card |
| `Z` | Expand all collapsed cards |
| `t` | Open selected session in a new detail tab |
| `X` | Kill all stale sessions |
| `q` | Quit tmuxpulse |

### Focused View

| Key | Action |
| --- | --- |
| `Up / Down` | Scroll pane output |
| `Esc` | Unfocus (return to grid) |
| `Ctrl+C` | Unfocus; press again to quit |

### Tab Navigation

| Key | Action |
| --- | --- |
| `Shift+Right` | Next tab |
| `Shift+Left` | Previous tab |
| `Ctrl+W` | Close current tab (not overview) |

### Mouse Support

| Action | Effect |
| --- | --- |
| Click on card | Focus that session |
| Scroll wheel | Scroll pane output (when focused) |

## Plugin System

tmuxpulse has a built-in plugin system for extending functionality with external scripts. Plugins are plain directories with a `plugin.toml` manifest and an executable entry point -- no compilation, WASM, or dynamic libraries required.

```
~/.local/share/tmuxpulse/plugins/my-plugin/
├── plugin.toml              # Manifest (required)
└── run.sh                   # Plugin executable (entry point)
```

### Plugin Manifest

```toml
name = "session-monitor"
version = "0.1.0"
description = "Monitors session activity and sends notifications"
entry = "./run.sh"
hooks = ["on_snapshot", "on_event"]
min_version = "0.1.0"
```

### Protocol

Plugins communicate with tmuxpulse over JSON stdin/stdout. TmuxPulse sends one JSON object per line to the plugin's stdin; the plugin responds with one JSON object per line on stdout.

### Lifecycle Hooks

| Event | Trigger | Payload |
| --- | --- | --- |
| `init` | Plugin loaded at startup | `{"type": "init", "tmuxpulse_version": "0.1.0"}` |
| `on_snapshot` | Each tick with full snapshot | `{"type": "on_snapshot", "snapshot": {...}}` |
| `on_event` | Control-mode event fires | `{"type": "on_event", "event": "SessionCreated(...)"}` |
| `shutdown` | TmuxPulse exiting | `{"type": "shutdown"}` |

### Plugin Response

Plugins can return status lines and notifications:

```json
{"ok": true, "status": "3 active sessions", "notification": null}
```

| Field | Type | Description |
| --- | --- | --- |
| `ok` | bool | Whether the plugin handled the message successfully |
| `status` | string? | Optional status line text displayed in the TUI |
| `notification` | string? | Optional toast notification |
| `log` | string? | Optional log message (written to tracing) |

### Plugin CLI

```bash
tmuxpulse plugin list              # List installed plugins
tmuxpulse plugin install ./my-plugin  # Install a plugin from a local directory
```

Plugin search directories (in order):
1. Paths listed in `[plugins] directories` in config
2. `~/.local/share/tmuxpulse/plugins/`
3. `~/.config/tmuxpulse/plugins/`

## Daemon RPC

tmuxpulse includes a daemon mode that exposes a JSON-RPC 2.0 API over a Unix domain socket. This enables AI agents, scripts, and external tools to query tmux state and send commands without parsing tmux output.

### Starting the Daemon

```bash
# Start with default socket path
tmuxpulse daemon start

# Start with custom socket
tmuxpulse daemon start --socket /tmp/my-tmuxpulse.sock

# Check status
tmuxpulse daemon status

# Stop
tmuxpulse daemon stop
```

### RPC Methods

| Method | Description | Required Params |
| --- | --- | --- |
| `pulse.ping` | Health check | -- |
| `pulse.version` | Version and uptime | -- |
| `pulse.snapshot` | Full tmux snapshot (sessions, windows, panes) | -- |
| `pulse.sessions` | List session names, IDs, and counts | -- |
| `pulse.capture` | Capture pane output | `pane_id`, optional `lines` |
| `pulse.send_keys` | Send keys to a pane | `pane_id`, `keys` |
| `pulse.kill_session` | Kill a session | `session_id` |

### Usage Examples

```bash
# From the CLI
tmuxpulse daemon call pulse.ping
tmuxpulse daemon call pulse.sessions
tmuxpulse daemon call pulse.capture '{"pane_id": "%1", "lines": 100}'
tmuxpulse daemon call pulse.send_keys '{"pane_id": "%1", "keys": ["ls", "Enter"]}'

# From any language via Unix socket (NDJSON protocol)
echo '{"jsonrpc":"2.0","method":"pulse.snapshot","params":{},"id":1}' | socat - UNIX-CONNECT:/tmp/tmuxpulse-1000.sock
```

### Security

- Socket permissions are set to `0600` (owner-only)
- Optional `auth_token` in config for bearer-style authentication
- Daemon refuses to start if another instance is already running on the same socket

## Configuration

tmuxpulse reads configuration from `~/.config/tmuxpulse/config.toml`. Generate the default file with `tmuxpulse config init`:

```toml
[general]
theme = "default"              # default | catppuccin-mocha | dracula | nord
poll_interval_ms = 1000        # Fallback polling interval in ms
capture_lines = 200            # Lines to capture per pane
stale_threshold_secs = 3600    # Seconds of inactivity before marking session stale

[ui]
show_hidden = false            # Show hidden sessions on startup
default_view = "grid"          # grid | detail
mouse = true                   # Enable mouse support
border_style = "rounded"       # rounded | plain | double | thick
show_status_bar = true         # Show bottom status bar
card_min_width = 40            # Minimum card width in columns
card_min_height = 12           # Minimum card height in rows

[keybindings]
quit = "q"
search = "/"
palette = "ctrl+p"
maximize = "z"
collapse = "c"
kill_stale = "X"
next_tab = "shift+right"
prev_tab = "shift+left"

[daemon]
socket_path = "/tmp/tmuxpulse.sock"   # Unix socket for RPC
auth_token = "auto"                    # Bearer token ("auto" = no auth)

[plugins]
enabled = []                   # List of enabled plugin names
directories = []               # Additional plugin search directories
```

All fields have sensible defaults -- **zero-config startup works out of the box**. CLI flags override config values, which override defaults.

## Theme Engine

tmuxpulse ships with 4 built-in themes that control all border, text, and UI colors:

| Theme | Style | Best For |
| --- | --- | --- |
| `default` | 256-color terminal palette | Universal compatibility |
| `catppuccin-mocha` | Warm pastel on dark background | Modern terminals with RGB support |
| `dracula` | Bold neon on dark purple | High-contrast preference |
| `nord` | Cool arctic blues | Minimal, calm aesthetic |

Switch themes via CLI or config:

```bash
tmuxpulse --theme dracula
```

```toml
# ~/.config/tmuxpulse/config.toml
[general]
theme = "catppuccin-mocha"
```

Each theme defines 20 color slots (borders, backgrounds, foregrounds, accents, status indicators). Custom themes can be defined in the config file in a future release.

## How It Works

1. `tmuxpulse` starts by calling `tmux list-sessions`, `list-windows`, and `list-panes` with format strings to build a typed snapshot of all sessions, windows, and panes.

2. A control mode client (`tmux -C attach`) provides an event stream for real-time change detection. Events like `%session-created`, `%window-add`, `%output`, and `%layout-change` trigger targeted refreshes instead of full-state polling.

3. For each visible session card, `tmux capture-pane -p -J` retrieves the active pane's latest output. A budget scheduler limits captures to 6 per tick with priority: focused session > cursor session > round-robin for the rest.

4. Captured content is hashed (FNV-1a) and compared against the previous hash. If unchanged, the render is skipped. If changed, the viewport updates and a 1.5-second border pulse animation triggers.

5. The Ratatui TUI renders an adaptive grid of session cards, a tab strip, and a status bar. Crossterm handles raw terminal I/O and mouse events.

6. User input is processed through a single-state Elm-style architecture: `AppState` + `handle_key_event()` + `draw_ui()`.

7. The daemon (when started) runs a background snapshot refresh loop and serves JSON-RPC requests over a Unix socket, enabling AI agents and scripts to interact with tmux programmatically.

## Architecture

```
                      tmux -C attach (control mode events)
+-------------+       tmux list-sessions -F "..."       +--------------+
|  tmuxpulse  | -----> tmux list-windows -F "..."  ----> | tmux server  |
|             | <----- tmux capture-pane -p -J     <---- |              |
| +---------+ |       tmux send-keys                     +--------------+
| | Config  | |       tmux kill-session
| +---------+ |
| | State   | |       Unix Socket (JSON-RPC 2.0)
| +---------+ |       +----------------+
| | Plugins | | <---> |  AI Agents /   |
| +---------+ |       |  Scripts       |
| | Daemon  | |       +----------------+
| +---------+ |
| | Ratatui | |
| +---------+ |
| | Terminal| |
| +---------+ |
+-------------+
```

## Project Structure

```
src/
├── lib.rs                  # Library crate root (public API for integration tests)
├── main.rs                 # CLI entry point, command dispatch, duration parsing
├── cli.rs                  # Command-line argument definitions (clap v4 derive)
├── config/
│   ├── mod.rs              # TOML config loading, defaults, validation
│   └── theme.rs            # Theme engine with 4 built-in themes (20 color slots each)
├── mux/
│   ├── mod.rs              # Core types: Session, Window, Pane, Snapshot, MuxEvent
│   └── tmux/
│       ├── mod.rs          # tmux client: snapshot, capture, send-keys, kill
│       ├── parser.rs       # Tab-separated format string parsing
│       └── control.rs      # tmux control mode client (event-driven monitoring)
├── plugin/
│   └── mod.rs              # Plugin system: TOML manifests, JSON protocol, lifecycle hooks
├── daemon/
│   └── mod.rs              # Daemon RPC: JSON-RPC 2.0 over Unix socket
├── state/
│   └── mod.rs              # Elm-style app state, FNV-1a content hashing, card states
└── ui/
    ├── mod.rs              # Ratatui app loop, input handling, overlays (search, palette, toast)
    ├── cards.rs            # Session card widget (pulse, stale, collapsed, focused states)
    ├── layout.rs           # Adaptive grid calculation (columns x rows from terminal size)
    ├── tabs.rs             # Tab strip widget (overview + per-session detail tabs)
    └── status.rs           # Status bar widget (session/pane count, stale count, shortcuts)
tests/
└── snapshots.rs            # Insta snapshot tests for serialization stability (5 tests)
.github/workflows/
├── ci.yml                  # CI: check, fmt, clippy, test (Linux + macOS), build, MSRV
└── release.yml             # Release: cross-compile 4 targets, GitHub Release with artifacts
```

## Security & Threat Model

tmuxpulse is designed for **single-user, local-only** use on development machines. The following controls are in place:

| Layer | Control | Detail |
| --- | --- | --- |
| **tmux access** | Local process only | Communicates with tmux via subprocess (`tmux list-sessions`, etc.); no network I/O |
| **RPC transport** | Unix socket + Bearer token | Socket at `$XDG_RUNTIME_DIR/tmuxpulse.sock` with `0600` permissions; optional auth token per request |
| **Config file** | Owner-only path | `~/.config/tmuxpulse/config.toml` follows XDG conventions |
| **Workspace snapshots** | Owner-only directory | Saved to `~/.local/share/tmuxpulse/workspaces/` with standard user permissions |
| **Plugin system** | Path traversal prevention | Plugin entry paths are resolved relative to the plugin directory; `..` segments in entry paths are rejected |
| **tmux commands** | No shell injection | All tmux arguments are passed as separate `&str` to `Command::new()`, never concatenated into a shell string |
| **Daemon startup** | Single-instance guard | Daemon checks for existing socket and refuses to start if another instance is running |

### Not recommended for

- **Multi-user / shared machines** -- Other local users with root or same-UID access could read tmux sessions. Restrict access via OS-level permissions or containers.
- **Untrusted tmux sessions** -- tmuxpulse captures and displays pane output as-is. Malicious terminal escape sequences in pane output could affect your terminal.
- **Production monitoring** -- tmuxpulse is a development tool. For production workloads, use dedicated monitoring infrastructure.

## Troubleshooting

### tmux not found

```
Error: tmux not found in PATH. Install tmux:
  macOS: brew install tmux
  Ubuntu/Debian: sudo apt install tmux
  Fedora: sudo dnf install tmux
```

Or specify the binary path explicitly:

```bash
tmuxpulse --tmux /usr/local/bin/tmux
```

### No sessions found

tmuxpulse requires at least one running tmux session:

```bash
tmux new -s dev -d    # Create a detached session
tmuxpulse             # Now it will show the session
```

### Terminal rendering issues

If the TUI renders incorrectly, try:

1. Ensure your terminal supports 256 colors (`echo $TERM` should show `xterm-256color` or similar)
2. Try a different theme: `tmuxpulse --theme default`
3. Resize your terminal to at least 80x24

### Config file errors

If the config file has syntax errors, tmuxpulse falls back to defaults and logs a warning to stderr. To reset:

```bash
rm ~/.config/tmuxpulse/config.toml
tmuxpulse config init
```

### Daemon issues

```bash
# Check if daemon is running
tmuxpulse daemon status

# If socket is stale (daemon crashed), remove it
rm /tmp/tmuxpulse-*.sock
tmuxpulse daemon start
```

## Roadmap

| Phase | Feature | Status |
| --- | --- | --- |
| 1 | Core TUI (grid, cards, tabs, search, palette) | Done |
| 1 | tmux client (snapshot, capture, send-keys, kill) | Done |
| 1 | TOML configuration + 4 built-in themes | Done |
| 1 | Workspace save | Done |
| 2 | tmux control mode (event-driven, <10ms latency) | Done |
| 2 | Plugin system (TOML manifest + JSON protocol + hooks) | Done |
| 2 | Daemon mode + JSON-RPC 2.0 server (Unix socket) | Done |
| 2 | GitHub Actions CI/CD (Linux + macOS matrix) | Done |
| 2 | Insta snapshot tests for serialization stability | Done |
| 3 | Workspace restore (recreate sessions/windows/panes) | Planned |
| 3 | Fuzzy search (skim integration) | Planned |
| 3 | AI agent SDK (Node.js + Python, zero dependencies) | Planned |
| 3 | Zellij backend (Multiplexer trait) | Planned |
| 4 | Shell completions (bash/zsh/fish) | Planned |
| 4 | Man page generation | Planned |
| 4 | Homebrew tap + crates.io publishing | Planned |

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for release history.

## Acknowledgments

Inspired by [steipete/tmuxwatch](https://github.com/steipete/tmuxwatch) and [justinhuangcode/browsercli](https://github.com/justinhuangcode/browsercli).

## License

[MIT](LICENSE)
