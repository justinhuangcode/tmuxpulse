# Contributing to TmuxPulse

Thank you for your interest in contributing to TmuxPulse! This guide will help you get started.

## Development Setup

### Prerequisites

- **Rust 1.88+** -- install via [rustup](https://rustup.rs/)
- **tmux 3.1+** -- install via your package manager
- **Git** -- for version control

### Getting Started

```bash
git clone https://github.com/justinhuangcode/tmuxpulse.git
cd tmuxpulse
cargo build
```

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run a specific test
cargo test test_name
```

### Code Quality

Before submitting a PR, ensure all checks pass:

```bash
# Format check
cargo fmt --check

# Lint check
cargo clippy --all-targets

# Full test suite
cargo test
```

## How to Contribute

### Reporting Bugs

1. Check [existing issues](https://github.com/justinhuangcode/tmuxpulse/issues) to avoid duplicates
2. Open a new issue with:
   - Clear title describing the bug
   - Steps to reproduce
   - Expected vs actual behavior
   - tmux version (`tmux -V`), OS, and terminal emulator

### Suggesting Features

1. Open an issue with the `enhancement` label
2. Describe the use case and why it would be valuable
3. Include mockups or examples if applicable

### Submitting Pull Requests

1. **Fork** the repository
2. **Create a branch** from `main`:
   ```bash
   git checkout -b feat/my-feature
   ```
3. **Make your changes** following the code style below
4. **Add tests** for new functionality
5. **Run all checks**:
   ```bash
   cargo fmt
   cargo clippy --all-targets
   cargo test
   ```
6. **Commit** with a clear message:
   ```
   feat: add fuzzy search for session names
   fix: correct pane capture offset calculation
   docs: update plugin development guide
   ```
7. **Push** and open a PR against `main`

### Commit Message Convention

We follow [Conventional Commits](https://www.conventionalcommits.org/):

| Prefix | Usage |
| --- | --- |
| `feat:` | New feature |
| `fix:` | Bug fix |
| `docs:` | Documentation changes |
| `refactor:` | Code refactoring (no behavior change) |
| `test:` | Adding or updating tests |
| `chore:` | Build, CI, or tooling changes |
| `perf:` | Performance improvement |

## Code Style

- Follow standard Rust conventions (`cargo fmt` enforces formatting)
- Use `anyhow::Result` for error propagation in application code
- Use `thiserror` for library error types
- Keep functions focused and under 50 lines where possible
- Add doc comments for public API items
- Prefer `&str` over `String` in function parameters when ownership is not needed

## Architecture Overview

```
src/
├── lib.rs          -- Library crate root (public API for integration tests)
├── main.rs         -- Binary entry point, CLI dispatch
├── cli.rs          -- clap v4 argument definitions
├── config/         -- TOML configuration, theme engine
├── mux/            -- tmux abstraction layer (snapshot, capture, control mode)
├── plugin/         -- Plugin system (TOML manifest, JSON protocol)
├── daemon/         -- JSON-RPC 2.0 daemon over Unix socket
├── state/          -- Elm-style application state, content hashing
└── ui/             -- Ratatui TUI (cards, grid layout, tabs, status bar)
```

Key principles:
- **Elm architecture**: Single `AppState` + event handling + pure rendering
- **Multiplexer abstraction**: `mux/` module is designed for future Zellij support
- **Budget-based capture**: Max 6 pane captures per tick with priority scheduling

## Testing

- **Unit tests**: In each module (`#[cfg(test)]` blocks)
- **Integration tests**: `tests/snapshots.rs` using Insta for serialization stability
- **CI matrix**: Tests run on both Linux and macOS

When adding new serializable types, add an Insta snapshot test to ensure format stability.

## Questions?

Open an issue or start a discussion. We're happy to help!
