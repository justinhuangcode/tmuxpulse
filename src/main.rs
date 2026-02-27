#![allow(dead_code)]

mod cli;
mod ui;

use anyhow::{Context, Result};
use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::cli::{Cli, Commands, ConfigAction, DaemonAction, WorkspaceAction};
use tmuxpulse::config::AppConfig;
use tmuxpulse::daemon;
use tmuxpulse::mux;
use tmuxpulse::mux::tmux::TmuxClient;
use tmuxpulse::plugin;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    // Load config
    let mut config =
        AppConfig::load(cli.config.as_deref()).context("failed to load configuration")?;

    // CLI overrides
    if let Some(ref theme) = cli.theme {
        config.general.theme = theme.clone();
    }
    if let Some(ref tmux_path) = cli.tmux {
        config.general.tmux_binary = Some(tmux_path.to_string_lossy().to_string());
    }
    if let Ok(interval) = parse_duration(&cli.interval) {
        config.general.poll_interval_ms = interval;
    }

    // Handle subcommands
    match cli.command {
        Some(Commands::Config { action }) => {
            return handle_config_command(action, &config);
        }
        Some(Commands::Workspace { action }) => {
            let client = TmuxClient::new(cli.tmux)?;
            return handle_workspace_command(action, &client).await;
        }
        Some(Commands::Plugin { action }) => {
            return handle_plugin_command(action, &config);
        }
        Some(Commands::Daemon { action }) => {
            return handle_daemon_command(action, &config, cli.tmux).await;
        }
        None => {}
    }

    // Create tmux client
    let tmux_bin = cli.tmux.or_else(|| {
        config
            .general
            .tmux_binary
            .as_ref()
            .map(std::path::PathBuf::from)
    });
    let client = TmuxClient::new(tmux_bin)?;

    // Dump mode: print JSON snapshot and exit
    if cli.dump {
        let snapshot = client.snapshot().await?;
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&snapshot)?);
        } else {
            print_snapshot_text(&snapshot);
        }
        return Ok(());
    }

    // Run TUI
    ui::run(config, client).await
}

fn handle_config_command(action: ConfigAction, config: &AppConfig) -> Result<()> {
    match action {
        ConfigAction::Init => {
            let path = AppConfig::default_path();
            if path.exists() {
                eprintln!("Config already exists at: {}", path.display());
                eprintln!("Delete it first to regenerate.");
                return Ok(());
            }
            AppConfig::write_default(&path)?;
            println!("Config written to: {}", path.display());
        }
        ConfigAction::Show => {
            let toml_str = toml::to_string_pretty(config)?;
            println!("{}", toml_str);
        }
    }
    Ok(())
}

async fn handle_workspace_command(action: WorkspaceAction, client: &TmuxClient) -> Result<()> {
    match action {
        WorkspaceAction::Save { name } => {
            let snapshot = client.snapshot().await?;
            let workspace_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("tmuxpulse")
                .join("workspaces");
            std::fs::create_dir_all(&workspace_dir)?;

            let path = workspace_dir.join(format!("{}.json", name));
            let json = serde_json::to_string_pretty(&snapshot)?;
            std::fs::write(&path, json)?;
            println!("Workspace '{}' saved to: {}", name, path.display());
        }
        WorkspaceAction::Restore { name } => {
            let workspace_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("tmuxpulse")
                .join("workspaces");
            let path = workspace_dir.join(format!("{}.json", name));
            if !path.exists() {
                eprintln!("Workspace '{}' not found at: {}", name, path.display());
                return Ok(());
            }
            println!(
                "Workspace restore not yet implemented. Snapshot at: {}",
                path.display()
            );
        }
        WorkspaceAction::List => {
            let workspace_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("tmuxpulse")
                .join("workspaces");
            if !workspace_dir.exists() {
                println!("No workspaces saved yet.");
                return Ok(());
            }
            let entries = std::fs::read_dir(&workspace_dir)?;
            let mut found = false;
            for entry in entries {
                let entry = entry?;
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".json") {
                        println!("  {}", name.trim_end_matches(".json"));
                        found = true;
                    }
                }
            }
            if !found {
                println!("No workspaces saved yet.");
            }
        }
    }
    Ok(())
}

fn handle_plugin_command(action: cli::PluginAction, config: &AppConfig) -> Result<()> {
    match action {
        cli::PluginAction::List => {
            let mut dirs = config.plugins.directories.clone();
            dirs.extend(plugin::default_plugin_dirs());

            let plugins = plugin::discover_plugins(&dirs);
            if plugins.is_empty() {
                println!("No plugins installed.");
                println!();
                println!("To install a plugin, place it in one of these directories:");
                for dir in &dirs {
                    println!("  {}", dir.display());
                }
                println!();
                println!("Each plugin must have a plugin.toml manifest.");
            } else {
                println!("Installed plugins:");
                for (path, manifest) in &plugins {
                    println!(
                        "  {} v{} - {} ({})",
                        manifest.name,
                        manifest.version,
                        if manifest.description.is_empty() {
                            "(no description)"
                        } else {
                            &manifest.description
                        },
                        path.display()
                    );
                }
            }
        }
        cli::PluginAction::Install { path } => {
            if !path.exists() {
                eprintln!("Plugin path does not exist: {}", path.display());
                return Ok(());
            }
            let manifest_path = path.join("plugin.toml");
            if !manifest_path.exists() {
                eprintln!("No plugin.toml found in {}", path.display());
                return Ok(());
            }

            // Copy plugin to default plugin directory
            let target_dir = plugin::default_plugin_dirs()
                .into_iter()
                .next()
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            std::fs::create_dir_all(&target_dir)?;

            let plugin_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let dest = target_dir.join(&plugin_name);

            if dest.exists() {
                eprintln!(
                    "Plugin '{}' already installed at {}",
                    plugin_name,
                    dest.display()
                );
                return Ok(());
            }

            // Copy directory recursively
            copy_dir_recursive(&path, &dest)?;
            println!("Plugin '{}' installed to {}", plugin_name, dest.display());
        }
    }
    Ok(())
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

async fn handle_daemon_command(
    action: DaemonAction,
    config: &AppConfig,
    tmux_path: Option<std::path::PathBuf>,
) -> Result<()> {
    let socket_path = config
        .daemon
        .socket_path
        .as_ref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(daemon::DaemonConfig::default_socket_path);

    match action {
        DaemonAction::Start { socket } => {
            let socket = socket.unwrap_or(socket_path);
            let client = TmuxClient::new(tmux_path)?;

            let auth_token = if config.daemon.auth_token == "auto" {
                None
            } else {
                Some(config.daemon.auth_token.clone())
            };

            let daemon_config = daemon::DaemonConfig {
                socket_path: socket,
                auth_token,
            };

            println!(
                "Starting daemon on {}...",
                daemon_config.socket_path.display()
            );
            daemon::start_daemon(daemon_config, client).await?;
        }
        DaemonAction::Status => {
            if daemon::is_daemon_running(&socket_path).await {
                let resp =
                    daemon::rpc_call(&socket_path, "pulse.version", serde_json::json!({})).await?;
                if let Some(result) = resp.result {
                    println!("Daemon running on {}", socket_path.display());
                    println!(
                        "  Version: {}",
                        result
                            .get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?")
                    );
                    println!(
                        "  Uptime: {}s",
                        result
                            .get("uptime_secs")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0)
                    );
                }
            } else {
                println!("Daemon not running.");
                println!("Start with: tmuxpulse daemon start");
            }
        }
        DaemonAction::Stop => {
            if !socket_path.exists() {
                println!("No daemon socket found.");
                return Ok(());
            }
            // Remove the socket file to signal shutdown
            std::fs::remove_file(&socket_path)?;
            println!("Daemon socket removed: {}", socket_path.display());
            println!("The daemon process will exit on next connection attempt.");
        }
        DaemonAction::Call { method, params } => {
            let params: serde_json::Value =
                serde_json::from_str(&params).context("invalid JSON params")?;

            let resp = daemon::rpc_call(&socket_path, &method, params).await?;

            if let Some(result) = resp.result {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else if let Some(error) = resp.error {
                eprintln!("RPC error ({}): {}", error.code, error.message);
            }
        }
    }
    Ok(())
}

fn print_snapshot_text(snapshot: &mux::Snapshot) {
    println!("tmuxpulse snapshot ({} sessions)", snapshot.sessions.len());
    println!("{}", "-".repeat(60));
    for session in &snapshot.sessions {
        let status = if session.attached {
            "attached"
        } else {
            "detached"
        };
        let stale = if session.is_stale(3600) {
            " [STALE]"
        } else {
            ""
        };
        println!("  {} ({}){}", session.name, status, stale);
        for window in &session.windows {
            let active = if window.active { "*" } else { " " };
            println!("    {}[{}] {}", active, window.index, window.name);
            for pane in &window.panes {
                let pane_active = if pane.active { ">" } else { " " };
                let dead = if pane.dead { " [DEAD]" } else { "" };
                println!(
                    "      {} {} ({}x{}) {}{}",
                    pane_active,
                    pane.current_command,
                    pane.width,
                    pane.height,
                    pane.current_path,
                    dead
                );
            }
        }
    }
}

/// Parse a duration string like "1s", "500ms", "2m"
fn parse_duration(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if let Some(ms) = s.strip_suffix("ms") {
        ms.parse::<u64>().map_err(|e| e.to_string())
    } else if let Some(secs) = s.strip_suffix('s') {
        secs.parse::<u64>()
            .map(|v| v * 1000)
            .map_err(|e| e.to_string())
    } else if let Some(mins) = s.strip_suffix('m') {
        mins.parse::<u64>()
            .map(|v| v * 60_000)
            .map_err(|e| e.to_string())
    } else {
        s.parse::<u64>().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_milliseconds() {
        assert_eq!(parse_duration("500ms").unwrap(), 500);
    }

    #[test]
    fn parse_duration_seconds() {
        assert_eq!(parse_duration("1s").unwrap(), 1000);
        assert_eq!(parse_duration("2s").unwrap(), 2000);
    }

    #[test]
    fn parse_duration_minutes() {
        assert_eq!(parse_duration("1m").unwrap(), 60_000);
    }

    #[test]
    fn parse_duration_raw_number() {
        assert_eq!(parse_duration("1000").unwrap(), 1000);
    }
}
