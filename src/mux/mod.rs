pub mod tmux;

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique session identifier
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionId(pub String);

/// Unique window identifier
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct WindowId(pub String);

/// Unique pane identifier
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct PaneId(pub String);

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for PaneId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A terminal multiplexer session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub name: String,
    pub attached: bool,
    pub windows: Vec<Window>,
    pub created_at: i64,
    pub last_activity: i64,
}

/// A window within a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Window {
    pub id: WindowId,
    pub session_id: SessionId,
    pub name: String,
    pub index: u32,
    pub active: bool,
    pub panes: Vec<Pane>,
}

/// A pane within a window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pane {
    pub id: PaneId,
    pub window_id: WindowId,
    pub session_id: SessionId,
    pub index: u32,
    pub active: bool,
    pub width: u16,
    pub height: u16,
    pub current_command: String,
    pub current_path: String,
    pub pid: u32,
    pub dead: bool,
    pub last_activity: i64,
}

/// A snapshot of the entire multiplexer state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub sessions: Vec<Session>,
    pub timestamp: i64,
}

/// Events emitted by the multiplexer
#[derive(Debug, Clone)]
pub enum MuxEvent {
    SessionCreated(SessionId),
    SessionClosed(SessionId),
    SessionRenamed {
        id: SessionId,
        new_name: String,
    },
    WindowCreated {
        session_id: SessionId,
        window_id: WindowId,
    },
    WindowClosed {
        session_id: SessionId,
        window_id: WindowId,
    },
    PaneOutput {
        pane_id: PaneId,
    },
    LayoutChanged(SessionId),
    ClientAttached(SessionId),
    ClientDetached(SessionId),
}

impl Session {
    /// Check if session is stale (all panes dead, or no activity for threshold)
    pub fn is_stale(&self, stale_threshold_secs: u64) -> bool {
        let all_panes_dead = self.windows.iter().all(|w| w.panes.iter().all(|p| p.dead));
        if all_panes_dead {
            return true;
        }

        if !self.attached {
            let now = chrono::Utc::now().timestamp();
            let idle_secs = (now - self.last_activity).unsigned_abs();
            return idle_secs > stale_threshold_secs;
        }

        false
    }

    /// Get the active window
    pub fn active_window(&self) -> Option<&Window> {
        self.windows.iter().find(|w| w.active)
    }

    /// Get the active pane of the active window
    pub fn active_pane(&self) -> Option<&Pane> {
        self.active_window()
            .and_then(|w| w.panes.iter().find(|p| p.active))
    }
}

impl Snapshot {
    pub fn empty() -> Self {
        Self {
            sessions: Vec::new(),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn pane_count(&self) -> usize {
        self.sessions
            .iter()
            .flat_map(|s| &s.windows)
            .map(|w| w.panes.len())
            .sum()
    }
}
