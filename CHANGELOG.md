# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-02-28

### Added

- **Core TUI** -- Real-time tmux session monitoring with adaptive grid layout
- **Session cards** -- Auto-arranged cards showing live pane output with pulse animation on activity
- **Event-driven monitoring** -- tmux control mode (`tmux -C`) for sub-10ms event response
- **Incremental capture** -- FNV-1a content hashing to skip unchanged pane re-renders
- **Budget-based scheduling** -- Max 6 pane captures per tick with priority: focused > cursor > round-robin
- **Stale detection** -- Sessions with all dead or idle panes marked as stale, bulk kill with `X`
- **Keyboard + mouse navigation** -- Arrow keys, Enter to focus, mouse click/scroll support
- **Command palette** -- `Ctrl+P` for keyboard-navigable action menu
- **Tab system** -- Overview grid + per-session detail tabs with `Shift+Left/Right` switching
- **Search/filter** -- `/` for live search across session names, window names, and pane commands
- **Workspace snapshots** -- `tmuxpulse workspace save <name>` to persist tmux topology as JSON
- **Plugin system** -- TOML manifest + JSON stdin/stdout protocol with lifecycle hooks (init, on_snapshot, on_event, shutdown)
- **Daemon RPC** -- JSON-RPC 2.0 server over Unix domain socket with 7 methods (ping, version, snapshot, sessions, capture, send_keys, kill_session)
- **TOML configuration** -- All visual and behavioral parameters configurable via `~/.config/tmuxpulse/config.toml`
- **4 built-in themes** -- Default, Catppuccin Mocha, Dracula, Nord (20 color slots each)
- **JSON output** -- `--dump --json` for machine-readable snapshots
- **CI/CD** -- GitHub Actions with check, fmt, clippy, test (Linux + macOS), build, and MSRV verification
- **Insta snapshot tests** -- Serialization stability tests for config, snapshot, and plugin manifest formats
- **Cross-platform** -- macOS and Linux support (Windows via WSL)

[0.1.0]: https://github.com/justinhuangcode/tmuxpulse/releases/tag/v0.1.0
