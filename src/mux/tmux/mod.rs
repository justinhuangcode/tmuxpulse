pub mod control;
mod parser;

use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tracing::{debug, warn};

use super::{Pane, PaneId, Session, SessionId, Snapshot, Window, WindowId};

const TMUX_TIMEOUT: Duration = Duration::from_secs(2);

/// Client for interacting with the tmux server
#[derive(Debug, Clone)]
pub struct TmuxClient {
    binary: PathBuf,
}

impl TmuxClient {
    pub fn new(binary: Option<PathBuf>) -> Result<Self> {
        let binary = match binary {
            Some(b) => b,
            None => which_tmux()?,
        };
        Ok(Self { binary })
    }

    /// Take a full snapshot of all sessions, windows, and panes
    pub async fn snapshot(&self) -> Result<Snapshot> {
        let sessions_raw = self.run_tmux_format(
            &["list-sessions"],
            "#{session_id}\t#{session_name}\t#{session_attached}\t#{session_created}\t#{session_activity}",
        ).await;

        let sessions_raw = match sessions_raw {
            Ok(raw) => raw,
            Err(e) => {
                if is_no_server_error(&e) {
                    return Ok(Snapshot::empty());
                }
                return Err(e);
            }
        };

        let mut sessions = Vec::new();

        for line in sessions_raw.lines() {
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() < 5 {
                warn!("malformed session line: {}", line);
                continue;
            }

            let session_id = SessionId(fields[0].to_string());
            let windows = self.list_windows(&session_id).await?;

            sessions.push(Session {
                id: session_id,
                name: fields[1].to_string(),
                attached: fields[2] != "0",
                windows,
                created_at: fields[3].parse().unwrap_or(0),
                last_activity: fields[4].parse().unwrap_or(0),
            });
        }

        Ok(Snapshot {
            sessions,
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    /// List windows for a session
    async fn list_windows(&self, session_id: &SessionId) -> Result<Vec<Window>> {
        let raw = self
            .run_tmux_format(
                &["list-windows", "-t", &session_id.0],
                "#{window_id}\t#{window_name}\t#{window_index}\t#{window_active}",
            )
            .await?;

        let mut windows = Vec::new();

        for line in raw.lines() {
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() < 4 {
                continue;
            }

            let window_id = WindowId(fields[0].to_string());
            let panes = self.list_panes(session_id, &window_id).await?;

            windows.push(Window {
                id: window_id,
                session_id: session_id.clone(),
                name: fields[1].to_string(),
                index: fields[2].parse().unwrap_or(0),
                active: fields[3] != "0",
                panes,
            });
        }

        Ok(windows)
    }

    /// List panes for a window
    async fn list_panes(&self, session_id: &SessionId, window_id: &WindowId) -> Result<Vec<Pane>> {
        let raw = self.run_tmux_format(
            &["list-panes", "-t", &window_id.0],
            "#{pane_id}\t#{pane_index}\t#{pane_active}\t#{pane_width}\t#{pane_height}\t#{pane_current_command}\t#{pane_current_path}\t#{pane_pid}\t#{pane_dead}\t#{pane_last_activity}",
        ).await?;

        let mut panes = Vec::new();

        for line in raw.lines() {
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() < 10 {
                continue;
            }

            panes.push(Pane {
                id: PaneId(fields[0].to_string()),
                window_id: window_id.clone(),
                session_id: session_id.clone(),
                index: fields[1].parse().unwrap_or(0),
                active: fields[2] != "0",
                width: fields[3].parse().unwrap_or(80),
                height: fields[4].parse().unwrap_or(24),
                current_command: fields[5].to_string(),
                current_path: fields[6].to_string(),
                pid: fields[7].parse().unwrap_or(0),
                dead: fields[8] != "0",
                last_activity: fields[9].parse().unwrap_or(0),
            });
        }

        Ok(panes)
    }

    /// Capture pane output
    pub async fn capture_pane(&self, pane_id: &PaneId, lines: usize) -> Result<String> {
        let start_line = format!("-{}", lines);
        self.run_tmux(&[
            "capture-pane",
            "-p",
            "-J",
            "-t",
            &pane_id.0,
            "-S",
            &start_line,
        ])
        .await
    }

    /// Send keys to a pane
    pub async fn send_keys(&self, pane_id: &PaneId, keys: &[String]) -> Result<()> {
        let mut args = vec!["send-keys", "-t", &pane_id.0];
        let key_refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
        args.extend(key_refs);
        self.run_tmux(&args).await?;
        Ok(())
    }

    /// Kill a session
    pub async fn kill_session(&self, session_id: &SessionId) -> Result<()> {
        self.run_tmux(&["kill-session", "-t", &session_id.0])
            .await?;
        Ok(())
    }

    /// Kill a pane
    pub async fn kill_pane(&self, pane_id: &PaneId) -> Result<()> {
        self.run_tmux(&["kill-pane", "-t", &pane_id.0]).await?;
        Ok(())
    }

    /// Run a tmux command with a format string, returning the formatted output
    async fn run_tmux_format(&self, args: &[&str], format: &str) -> Result<String> {
        let mut full_args: Vec<&str> = args.to_vec();
        full_args.push("-F");
        full_args.push(format);
        self.run_tmux(&full_args).await
    }

    /// Run a tmux command and return stdout
    async fn run_tmux(&self, args: &[&str]) -> Result<String> {
        debug!(binary = %self.binary.display(), args = ?args, "running tmux command");

        let output = tokio::time::timeout(
            TMUX_TIMEOUT,
            Command::new(&self.binary)
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .context("failed to spawn tmux")?
                .wait_with_output(),
        )
        .await
        .context("tmux command timed out")?
        .context("tmux command failed")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("tmux error: {}", stderr.trim());
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

/// Find tmux binary in PATH
fn which_tmux() -> Result<PathBuf> {
    let candidates = ["tmux"];
    for name in &candidates {
        if let Ok(output) = std::process::Command::new("which").arg(name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return Ok(PathBuf::from(path));
            }
        }
    }
    bail!(
        "tmux not found in PATH. Install tmux:\n  \
         macOS: brew install tmux\n  \
         Ubuntu/Debian: sudo apt install tmux\n  \
         Fedora: sudo dnf install tmux"
    )
}

/// Check if an error indicates no tmux server is running
fn is_no_server_error(err: &anyhow::Error) -> bool {
    let msg = err.to_string();
    msg.contains("no server running")
        || msg.contains("no current client")
        || msg.contains("error connecting")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn which_tmux_returns_path_or_error() {
        // This test just verifies the function doesn't panic
        let _ = which_tmux();
    }

    #[test]
    fn no_server_error_detection() {
        let err = anyhow::anyhow!("no server running on /tmp/tmux-1000/default");
        assert!(is_no_server_error(&err));

        let err = anyhow::anyhow!("some other error");
        assert!(!is_no_server_error(&err));
    }
}
