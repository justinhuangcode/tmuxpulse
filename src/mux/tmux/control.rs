//! tmux control mode client.
//!
//! tmux control mode (`tmux -C`) provides a structured event stream
//! instead of polling. Events are emitted as `%event-name args` lines
//! on stdout. This module parses those events into typed MuxEvent values.
//!
//! Reference: https://man.openbsd.org/tmux#CONTROL_MODE

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::mux::{MuxEvent, PaneId, SessionId, WindowId};

/// A connection to tmux in control mode
pub struct ControlModeClient {
    child: Child,
    event_rx: mpsc::UnboundedReceiver<MuxEvent>,
}

impl ControlModeClient {
    /// Start tmux in control mode, attaching to an existing server.
    /// Returns a client that yields events as they arrive.
    pub async fn start(tmux_binary: &PathBuf) -> Result<Self> {
        let mut child = Command::new(tmux_binary)
            .args(["-C", "attach"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("failed to start tmux control mode")?;

        let stdout = child
            .stdout
            .take()
            .context("failed to capture tmux control mode stdout")?;

        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // Spawn a task to read and parse control mode output
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(event) = parse_control_line(&line) {
                    if event_tx.send(event).is_err() {
                        break; // receiver dropped
                    }
                }
            }
        });

        info!("tmux control mode started");

        Ok(Self { child, event_rx })
    }

    /// Receive the next event. Returns None if the control mode session ended.
    pub async fn next_event(&mut self) -> Option<MuxEvent> {
        self.event_rx.recv().await
    }

    /// Check if an event is available without blocking
    pub fn try_next_event(&mut self) -> Option<MuxEvent> {
        self.event_rx.try_recv().ok()
    }

    /// Drain all pending events
    pub fn drain_events(&mut self) -> Vec<MuxEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.event_rx.try_recv() {
            events.push(event);
        }
        events
    }

    /// Stop the control mode session
    pub async fn stop(&mut self) -> Result<()> {
        self.child
            .kill()
            .await
            .context("failed to kill tmux control mode")?;
        Ok(())
    }
}

impl Drop for ControlModeClient {
    fn drop(&mut self) {
        // Best-effort kill on drop
        let _ = self.child.start_kill();
    }
}

/// Parse a single control mode output line into a MuxEvent.
///
/// Control mode lines have the format:
///   %event-name [args...]
///
/// Known events:
///   %session-changed $id name
///   %session-renamed $id name
///   %session-created $id
///   %session-closed $id
///   %window-add @id
///   %window-close @id
///   %window-renamed @id name
///   %output %id content
///   %layout-change $id layout
///   %client-session-changed $id name
///   %sessions-changed
fn parse_control_line(line: &str) -> Option<MuxEvent> {
    let line = line.trim();

    if !line.starts_with('%') {
        return None;
    }

    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    let event_name = parts.first()?;

    match *event_name {
        "%session-created" => {
            let id = parts.get(1)?;
            Some(MuxEvent::SessionCreated(SessionId(id.to_string())))
        }
        "%session-closed" => {
            let id = parts.get(1)?;
            Some(MuxEvent::SessionClosed(SessionId(id.to_string())))
        }
        "%session-renamed" => {
            let id = parts.get(1)?;
            let name = parts.get(2).unwrap_or(&"");
            Some(MuxEvent::SessionRenamed {
                id: SessionId(id.to_string()),
                new_name: name.to_string(),
            })
        }
        "%window-add" => {
            let id = parts.get(1)?;
            Some(MuxEvent::WindowCreated {
                session_id: SessionId(String::new()), // Not provided in this event
                window_id: WindowId(id.to_string()),
            })
        }
        "%window-close" => {
            let id = parts.get(1)?;
            Some(MuxEvent::WindowClosed {
                session_id: SessionId(String::new()),
                window_id: WindowId(id.to_string()),
            })
        }
        "%output" => {
            let id = parts.get(1)?;
            Some(MuxEvent::PaneOutput {
                pane_id: PaneId(id.to_string()),
            })
        }
        "%layout-change" => {
            let id = parts.get(1)?;
            Some(MuxEvent::LayoutChanged(SessionId(id.to_string())))
        }
        "%client-session-changed" => {
            let id = parts.get(1)?;
            Some(MuxEvent::ClientAttached(SessionId(id.to_string())))
        }
        "%sessions-changed" => {
            // Generic sessions-changed event -- treat as a signal to refresh
            // We don't have a specific session ID, so use an empty one
            Some(MuxEvent::SessionCreated(SessionId(String::new())))
        }
        _ => {
            debug!("unknown control mode event: {}", event_name);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_session_created() {
        let event = parse_control_line("%session-created $3").unwrap();
        match event {
            MuxEvent::SessionCreated(id) => assert_eq!(id.0, "$3"),
            _ => panic!("expected SessionCreated"),
        }
    }

    #[test]
    fn parse_session_closed() {
        let event = parse_control_line("%session-closed $1").unwrap();
        match event {
            MuxEvent::SessionClosed(id) => assert_eq!(id.0, "$1"),
            _ => panic!("expected SessionClosed"),
        }
    }

    #[test]
    fn parse_session_renamed() {
        let event = parse_control_line("%session-renamed $2 new-name").unwrap();
        match event {
            MuxEvent::SessionRenamed { id, new_name } => {
                assert_eq!(id.0, "$2");
                assert_eq!(new_name, "new-name");
            }
            _ => panic!("expected SessionRenamed"),
        }
    }

    #[test]
    fn parse_window_add() {
        let event = parse_control_line("%window-add @5").unwrap();
        match event {
            MuxEvent::WindowCreated { window_id, .. } => assert_eq!(window_id.0, "@5"),
            _ => panic!("expected WindowCreated"),
        }
    }

    #[test]
    fn parse_output() {
        let event = parse_control_line("%output %7 hello world").unwrap();
        match event {
            MuxEvent::PaneOutput { pane_id } => assert_eq!(pane_id.0, "%7"),
            _ => panic!("expected PaneOutput"),
        }
    }

    #[test]
    fn parse_layout_change() {
        let event = parse_control_line("%layout-change $1 abc123").unwrap();
        match event {
            MuxEvent::LayoutChanged(id) => assert_eq!(id.0, "$1"),
            _ => panic!("expected LayoutChanged"),
        }
    }

    #[test]
    fn parse_unknown_event() {
        assert!(parse_control_line("%unknown-event foo").is_none());
    }

    #[test]
    fn parse_non_event_line() {
        assert!(parse_control_line("some regular output").is_none());
        assert!(parse_control_line("").is_none());
    }

    #[test]
    fn parse_client_session_changed() {
        let event = parse_control_line("%client-session-changed $2 dev").unwrap();
        match event {
            MuxEvent::ClientAttached(id) => assert_eq!(id.0, "$2"),
            _ => panic!("expected ClientAttached"),
        }
    }
}
