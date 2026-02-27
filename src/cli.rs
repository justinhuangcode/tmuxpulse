use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "tmuxpulse",
    about = "Real-time, event-driven TUI for monitoring and managing tmux sessions",
    version,
    author
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Poll interval fallback when control mode is unavailable (e.g. "1s", "500ms")
    #[arg(long, default_value = "1s")]
    pub interval: String,

    /// Path to tmux binary
    #[arg(long)]
    pub tmux: Option<PathBuf>,

    /// Dump JSON snapshot and exit
    #[arg(long)]
    pub dump: bool,

    /// Configuration file path
    #[arg(long, short)]
    pub config: Option<PathBuf>,

    /// Theme name
    #[arg(long)]
    pub theme: Option<String>,

    /// Output as JSON (machine-readable)
    #[arg(long)]
    pub json: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Manage workspace snapshots
    Workspace {
        #[command(subcommand)]
        action: WorkspaceAction,
    },
    /// Manage plugins
    Plugin {
        #[command(subcommand)]
        action: PluginAction,
    },
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Daemon RPC server for AI agents and scripts
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum WorkspaceAction {
    /// Save current tmux layout as a named workspace
    Save {
        /// Workspace name
        name: String,
    },
    /// Restore a saved workspace
    Restore {
        /// Workspace name
        name: String,
    },
    /// List saved workspaces
    List,
}

#[derive(Subcommand, Debug)]
pub enum PluginAction {
    /// List installed plugins
    List,
    /// Install a plugin from a path
    Install {
        /// Path to plugin directory
        path: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Generate default configuration file
    Init,
    /// Show current configuration
    Show,
}

#[derive(Subcommand, Debug)]
pub enum DaemonAction {
    /// Start the daemon RPC server
    Start {
        /// Custom socket path
        #[arg(long)]
        socket: Option<PathBuf>,
    },
    /// Check daemon status
    Status,
    /// Stop a running daemon
    Stop,
    /// Send an RPC call to the daemon
    Call {
        /// RPC method (e.g. pulse.snapshot)
        method: String,
        /// JSON params (optional)
        #[arg(default_value = "{}")]
        params: String,
    },
}
