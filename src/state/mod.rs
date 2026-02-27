use std::collections::HashMap;
use std::time::Instant;

use crate::config::AppConfig;
use crate::mux::{PaneId, Session, SessionId, Snapshot};

/// Captured pane content with change tracking
#[derive(Debug, Clone)]
pub struct PanePreview {
    pub content: String,
    pub content_hash: u64,
    pub last_changed: Instant,
    pub scroll_offset: u16,
    pub at_bottom: bool,
}

impl Default for PanePreview {
    fn default() -> Self {
        Self::new()
    }
}

impl PanePreview {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            content_hash: 0,
            last_changed: Instant::now(),
            scroll_offset: 0,
            at_bottom: true,
        }
    }

    /// Update content, returns true if changed
    pub fn update(&mut self, new_content: &str) -> bool {
        let new_hash = fast_hash(new_content);
        if new_hash != self.content_hash {
            self.content_hash = new_hash;
            self.content = new_content.to_string();
            self.last_changed = Instant::now();
            if self.at_bottom {
                // Auto-scroll will be handled in rendering
                self.scroll_offset = u16::MAX;
            }
            true
        } else {
            false
        }
    }

    /// Check if this preview has recent activity (for pulse animation)
    pub fn is_pulsing(&self) -> bool {
        self.last_changed.elapsed().as_millis() < 1500
    }
}

/// Card display state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardState {
    Normal,
    Collapsed,
    Hidden,
    Maximized,
}

/// Tab in the tab strip
#[derive(Debug, Clone)]
pub enum Tab {
    Overview,
    Session(SessionId),
}

/// Application state -- single source of truth (Elm architecture)
pub struct AppState {
    pub config: AppConfig,
    pub snapshot: Snapshot,
    pub previews: HashMap<PaneId, PanePreview>,
    pub card_states: HashMap<SessionId, CardState>,

    // Navigation
    pub cursor_index: usize,
    pub focused_session: Option<SessionId>,
    pub hovered_session: Option<SessionId>,

    // Tabs
    pub tabs: Vec<Tab>,
    pub active_tab: usize,

    // Search
    pub search_active: bool,
    pub search_query: String,

    // Command palette
    pub palette_open: bool,
    pub palette_index: usize,

    // Toast notification
    pub toast_message: Option<String>,
    pub toast_expires: Option<Instant>,

    // Capture scheduling
    pub capture_offset: usize,

    // Status
    pub last_error: Option<String>,
    pub inflight: bool,
    pub tick_count: u64,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            snapshot: Snapshot::empty(),
            previews: HashMap::new(),
            card_states: HashMap::new(),
            cursor_index: 0,
            focused_session: None,
            hovered_session: None,
            tabs: vec![Tab::Overview],
            active_tab: 0,
            search_active: false,
            search_query: String::new(),
            palette_open: false,
            palette_index: 0,
            toast_message: None,
            toast_expires: None,
            capture_offset: 0,
            last_error: None,
            inflight: false,
            tick_count: 0,
        }
    }

    /// Get visible sessions (filtered by search and card state)
    pub fn visible_sessions(&self) -> Vec<&Session> {
        self.snapshot
            .sessions
            .iter()
            .filter(|s| {
                let card_state = self
                    .card_states
                    .get(&s.id)
                    .copied()
                    .unwrap_or(CardState::Normal);
                card_state != CardState::Hidden
            })
            .filter(|s| {
                if self.search_query.is_empty() {
                    return true;
                }
                session_matches(s, &self.search_query)
            })
            .collect()
    }

    /// Get the currently focused session
    pub fn focused(&self) -> Option<&Session> {
        self.focused_session
            .as_ref()
            .and_then(|id| self.snapshot.sessions.iter().find(|s| &s.id == id))
    }

    /// Toggle card state for a session
    pub fn toggle_collapse(&mut self, session_id: &SessionId) {
        let state = self
            .card_states
            .entry(session_id.clone())
            .or_insert(CardState::Normal);
        *state = match *state {
            CardState::Normal => CardState::Collapsed,
            CardState::Collapsed => CardState::Normal,
            other => other,
        };
    }

    /// Toggle maximize for a session
    pub fn toggle_maximize(&mut self, session_id: &SessionId) {
        let state = self
            .card_states
            .entry(session_id.clone())
            .or_insert(CardState::Normal);
        *state = match *state {
            CardState::Maximized => CardState::Normal,
            _ => CardState::Maximized,
        };
    }

    /// Show a toast notification
    pub fn show_toast(&mut self, message: String) {
        self.toast_message = Some(message);
        self.toast_expires = Some(Instant::now() + std::time::Duration::from_secs(3));
    }

    /// Check if toast has expired
    pub fn check_toast(&mut self) {
        if let Some(expires) = self.toast_expires {
            if Instant::now() >= expires {
                self.toast_message = None;
                self.toast_expires = None;
            }
        }
    }

    /// Open a session in a detail tab
    pub fn open_session_tab(&mut self, session_id: SessionId) {
        // Check if tab already exists
        for (i, tab) in self.tabs.iter().enumerate() {
            if let Tab::Session(id) = tab {
                if id == &session_id {
                    self.active_tab = i;
                    return;
                }
            }
        }
        self.tabs.push(Tab::Session(session_id));
        self.active_tab = self.tabs.len() - 1;
    }

    /// Close the active tab (if not overview)
    pub fn close_active_tab(&mut self) {
        if self.active_tab > 0 {
            self.tabs.remove(self.active_tab);
            self.active_tab = self.active_tab.saturating_sub(1);
        }
    }

    /// Update snapshot and reconcile state
    pub fn update_snapshot(&mut self, new_snapshot: Snapshot) {
        // Remove card states for sessions that no longer exist
        let existing_ids: Vec<SessionId> =
            new_snapshot.sessions.iter().map(|s| s.id.clone()).collect();
        self.card_states.retain(|id, _| existing_ids.contains(id));

        // Remove tabs for sessions that no longer exist
        self.tabs.retain(|tab| match tab {
            Tab::Overview => true,
            Tab::Session(id) => existing_ids.contains(id),
        });
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len().saturating_sub(1);
        }

        // Clamp cursor
        let visible_count = new_snapshot.sessions.len();
        if self.cursor_index >= visible_count && visible_count > 0 {
            self.cursor_index = visible_count - 1;
        }

        self.snapshot = new_snapshot;
        self.tick_count += 1;
    }

    /// Get palette actions
    pub fn palette_actions(&self) -> Vec<PaletteAction> {
        let mut actions = vec![
            PaletteAction::new("Refresh", "Force refresh snapshot"),
            PaletteAction::new("Show Hidden", "Show all hidden sessions"),
            PaletteAction::new("Expand All", "Expand all collapsed sessions"),
        ];

        let stale_count = self
            .snapshot
            .sessions
            .iter()
            .filter(|s| s.is_stale(self.config.general.stale_threshold_secs))
            .count();
        if stale_count > 0 {
            actions.push(PaletteAction::new(
                "Kill Stale",
                &format!("Kill {} stale sessions", stale_count),
            ));
        }

        actions
    }
}

#[derive(Debug, Clone)]
pub struct PaletteAction {
    pub name: String,
    pub description: String,
}

impl PaletteAction {
    fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
        }
    }
}

/// Check if a session matches a search query (name, window names, pane commands)
fn session_matches(session: &Session, query: &str) -> bool {
    let query_lower = query.to_lowercase();

    if session.name.to_lowercase().contains(&query_lower) {
        return true;
    }

    for window in &session.windows {
        if window.name.to_lowercase().contains(&query_lower) {
            return true;
        }
        for pane in &window.panes {
            if pane.current_command.to_lowercase().contains(&query_lower) {
                return true;
            }
        }
    }

    false
}

/// Fast non-cryptographic hash for content diffing
fn fast_hash(s: &str) -> u64 {
    // FNV-1a hash
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mux::*;

    fn make_session(name: &str) -> Session {
        Session {
            id: SessionId(format!("${}", name)),
            name: name.to_string(),
            attached: false,
            windows: vec![Window {
                id: WindowId("@1".to_string()),
                session_id: SessionId(format!("${}", name)),
                name: "bash".to_string(),
                index: 0,
                active: true,
                panes: vec![Pane {
                    id: PaneId("%1".to_string()),
                    window_id: WindowId("@1".to_string()),
                    session_id: SessionId(format!("${}", name)),
                    index: 0,
                    active: true,
                    width: 80,
                    height: 24,
                    current_command: "vim".to_string(),
                    current_path: "/home/user".to_string(),
                    pid: 1234,
                    dead: false,
                    last_activity: 0,
                }],
            }],
            created_at: 0,
            last_activity: 0,
        }
    }

    #[test]
    fn session_search_by_name() {
        let session = make_session("development");
        assert!(session_matches(&session, "dev"));
        assert!(session_matches(&session, "DEV"));
        assert!(!session_matches(&session, "prod"));
    }

    #[test]
    fn session_search_by_command() {
        let session = make_session("dev");
        assert!(session_matches(&session, "vim"));
        assert!(!session_matches(&session, "emacs"));
    }

    #[test]
    fn pane_preview_tracks_changes() {
        let mut preview = PanePreview::new();
        assert!(preview.update("hello"));
        assert!(!preview.update("hello")); // same content
        assert!(preview.update("world")); // different content
    }

    #[test]
    fn fast_hash_deterministic() {
        assert_eq!(fast_hash("hello"), fast_hash("hello"));
        assert_ne!(fast_hash("hello"), fast_hash("world"));
    }

    #[test]
    fn card_state_toggle() {
        let config = AppConfig::default();
        let mut state = AppState::new(config);
        let id = SessionId("$1".to_string());

        assert_eq!(state.card_states.get(&id), None);
        state.toggle_collapse(&id);
        assert_eq!(state.card_states.get(&id), Some(&CardState::Collapsed));
        state.toggle_collapse(&id);
        assert_eq!(state.card_states.get(&id), Some(&CardState::Normal));
    }
}
