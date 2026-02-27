mod cards;
mod layout;
mod status;
mod tabs;

use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Terminal;

use tmuxpulse::config::AppConfig;
use tmuxpulse::mux::tmux::TmuxClient;
use tmuxpulse::mux::SessionId;
use tmuxpulse::state::{AppState, CardState, Tab};

/// Run the TUI application
pub async fn run(config: AppConfig, client: TmuxClient) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, config, client).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: AppConfig,
    client: TmuxClient,
) -> Result<()> {
    let poll_interval = Duration::from_millis(config.general.poll_interval_ms);
    let capture_lines = config.general.capture_lines;
    let mut state = AppState::new(config);

    // Initial snapshot
    match client.snapshot().await {
        Ok(snapshot) => state.update_snapshot(snapshot),
        Err(e) => state.last_error = Some(format!("Initial snapshot failed: {}", e)),
    }

    loop {
        // Draw
        terminal.draw(|f| draw_ui(f, &mut state))?;

        // Poll for events with timeout (this is our tick mechanism)
        if event::poll(poll_interval)? {
            match event::read()? {
                Event::Key(key) => {
                    let action = handle_key_event(key, &mut state);
                    match action {
                        Action::Quit => return Ok(()),
                        Action::KillSession(id) => {
                            if let Err(e) = client.kill_session(&id).await {
                                state.last_error = Some(format!("Failed to kill session: {}", e));
                            } else {
                                state.show_toast("Session killed".to_string());
                            }
                        }
                        Action::KillStale => {
                            let stale: Vec<SessionId> = state
                                .snapshot
                                .sessions
                                .iter()
                                .filter(|s| s.is_stale(state.config.general.stale_threshold_secs))
                                .map(|s| s.id.clone())
                                .collect();
                            let count = stale.len();
                            for id in stale {
                                let _ = client.kill_session(&id).await;
                            }
                            state.show_toast(format!("{} stale sessions killed", count));
                        }
                        Action::SendKeys(pane_id, keys) => {
                            let _ = client.send_keys(&pane_id, &keys).await;
                        }
                        Action::None => {}
                    }
                }
                Event::Mouse(mouse) => {
                    handle_mouse_event(mouse, &mut state);
                }
                Event::Resize(_, _) => {
                    // Terminal resize is handled automatically by ratatui
                }
                _ => {}
            }
        }

        // Tick: refresh snapshot
        if !state.inflight {
            state.inflight = true;
            match client.snapshot().await {
                Ok(snapshot) => {
                    state.update_snapshot(snapshot);
                    state.last_error = None;
                }
                Err(e) => {
                    state.last_error = Some(format!("{}", e));
                }
            }

            // Capture pane content for visible sessions
            let capture_budget = 6usize;

            // Priority: focused > cursor > round-robin
            let mut pane_ids_to_capture: Vec<crate::mux::PaneId> = Vec::new();

            // Focused session first
            if let Some(focused) = &state.focused_session {
                if let Some(session) = state.snapshot.sessions.iter().find(|s| &s.id == focused) {
                    if let Some(pane) = session.active_pane() {
                        pane_ids_to_capture.push(pane.id.clone());
                    }
                }
            }

            // Then cursor session
            let visible = state.visible_sessions();
            if let Some(cursor_session) = visible.get(state.cursor_index) {
                if let Some(pane) = cursor_session.active_pane() {
                    if !pane_ids_to_capture.contains(&pane.id) {
                        pane_ids_to_capture.push(pane.id.clone());
                    }
                }
            }

            // Round-robin for the rest
            let all_active_panes: Vec<crate::mux::PaneId> = state
                .snapshot
                .sessions
                .iter()
                .filter(|s| {
                    let card = state
                        .card_states
                        .get(&s.id)
                        .copied()
                        .unwrap_or(CardState::Normal);
                    card != CardState::Collapsed && card != CardState::Hidden
                })
                .filter_map(|s| s.active_pane())
                .map(|p| p.id.clone())
                .collect();

            if !all_active_panes.is_empty() {
                for i in 0..all_active_panes.len() {
                    let idx = (state.capture_offset + i) % all_active_panes.len();
                    let pane_id = &all_active_panes[idx];
                    if !pane_ids_to_capture.contains(pane_id) {
                        pane_ids_to_capture.push(pane_id.clone());
                    }
                    if pane_ids_to_capture.len() >= capture_budget {
                        break;
                    }
                }
                state.capture_offset =
                    (state.capture_offset + capture_budget) % all_active_panes.len();
            }

            // Execute captures
            for pane_id in pane_ids_to_capture.iter().take(capture_budget) {
                if let Ok(content) = client.capture_pane(pane_id, capture_lines).await {
                    let preview = state.previews.entry(pane_id.clone()).or_default();
                    preview.update(&content);
                }
            }

            state.inflight = false;
        }

        // Check toast expiry
        state.check_toast();
    }
}

/// Actions produced by input handling
enum Action {
    None,
    Quit,
    KillSession(SessionId),
    KillStale,
    SendKeys(crate::mux::PaneId, Vec<String>),
}

fn handle_key_event(key: KeyEvent, state: &mut AppState) -> Action {
    // Search mode captures input
    if state.search_active {
        match key.code {
            KeyCode::Esc => {
                state.search_active = false;
                state.search_query.clear();
            }
            KeyCode::Enter => {
                state.search_active = false;
            }
            KeyCode::Backspace => {
                state.search_query.pop();
            }
            KeyCode::Char(c) => {
                state.search_query.push(c);
            }
            _ => {}
        }
        return Action::None;
    }

    // Command palette captures input
    if state.palette_open {
        let actions = state.palette_actions();
        match key.code {
            KeyCode::Esc => {
                state.palette_open = false;
            }
            KeyCode::Up => {
                state.palette_index = state.palette_index.saturating_sub(1);
            }
            KeyCode::Down => {
                if state.palette_index + 1 < actions.len() {
                    state.palette_index += 1;
                }
            }
            KeyCode::Enter => {
                state.palette_open = false;
                if let Some(action) = actions.get(state.palette_index) {
                    match action.name.as_str() {
                        "Show Hidden" => {
                            state.card_states.values_mut().for_each(|s| {
                                if *s == CardState::Hidden {
                                    *s = CardState::Normal;
                                }
                            });
                            state.show_toast("All sessions visible".to_string());
                        }
                        "Expand All" => {
                            state.card_states.values_mut().for_each(|s| {
                                if *s == CardState::Collapsed {
                                    *s = CardState::Normal;
                                }
                            });
                            state.show_toast("All sessions expanded".to_string());
                        }
                        "Kill Stale" => {
                            return Action::KillStale;
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        return Action::None;
    }

    // Global shortcuts
    match key.code {
        KeyCode::Char('q') if key.modifiers.is_empty() && state.focused_session.is_none() => {
            return Action::Quit;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if state.focused_session.is_some() {
                // First ctrl+c unfocuses
                state.focused_session = None;
            } else {
                return Action::Quit;
            }
        }
        KeyCode::Char('/') if state.focused_session.is_none() => {
            state.search_active = true;
            state.search_query.clear();
        }
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.palette_open = true;
            state.palette_index = 0;
        }
        KeyCode::Esc => {
            if state.focused_session.is_some() {
                state.focused_session = None;
            } else if state.active_tab > 0 {
                state.active_tab = 0;
            }
        }
        KeyCode::Enter if state.focused_session.is_none() => {
            let visible = state.visible_sessions();
            if let Some(session) = visible.get(state.cursor_index) {
                state.focused_session = Some(session.id.clone());
            }
        }
        // Navigation
        KeyCode::Left if state.focused_session.is_none() => {
            state.cursor_index = state.cursor_index.saturating_sub(1);
        }
        KeyCode::Right if state.focused_session.is_none() => {
            let max = state.visible_sessions().len().saturating_sub(1);
            if state.cursor_index < max {
                state.cursor_index += 1;
            }
        }
        KeyCode::Up if state.focused_session.is_none() => {
            // Move up by grid columns (calculated during render)
            state.cursor_index = state.cursor_index.saturating_sub(3);
        }
        KeyCode::Down if state.focused_session.is_none() => {
            let max = state.visible_sessions().len().saturating_sub(1);
            state.cursor_index = (state.cursor_index + 3).min(max);
        }
        // Scroll in focused session
        KeyCode::Up if state.focused_session.is_some() => {
            if let Some(ref focused_id) = state.focused_session {
                if let Some(session) = state.snapshot.sessions.iter().find(|s| &s.id == focused_id)
                {
                    if let Some(pane) = session.active_pane() {
                        if let Some(preview) = state.previews.get_mut(&pane.id) {
                            preview.scroll_offset = preview.scroll_offset.saturating_sub(1);
                            preview.at_bottom = false;
                        }
                    }
                }
            }
        }
        KeyCode::Down if state.focused_session.is_some() => {
            if let Some(ref focused_id) = state.focused_session {
                if let Some(session) = state.snapshot.sessions.iter().find(|s| &s.id == focused_id)
                {
                    if let Some(pane) = session.active_pane() {
                        if let Some(preview) = state.previews.get_mut(&pane.id) {
                            preview.scroll_offset = preview.scroll_offset.saturating_add(1);
                        }
                    }
                }
            }
        }
        // Card actions
        KeyCode::Char('z') if state.focused_session.is_none() => {
            let visible = state.visible_sessions();
            if let Some(session) = visible.get(state.cursor_index) {
                let id = session.id.clone();
                state.toggle_collapse(&id);
            }
        }
        KeyCode::Char('Z') => {
            // Expand all
            state.card_states.values_mut().for_each(|s| {
                if *s == CardState::Collapsed {
                    *s = CardState::Normal;
                }
            });
        }
        KeyCode::Char('X') if state.focused_session.is_none() => {
            return Action::KillStale;
        }
        // Tab navigation
        KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
            if state.active_tab + 1 < state.tabs.len() {
                state.active_tab += 1;
            }
        }
        KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => {
            state.active_tab = state.active_tab.saturating_sub(1);
        }
        KeyCode::Char('t') if state.focused_session.is_none() => {
            let visible = state.visible_sessions();
            if let Some(session) = visible.get(state.cursor_index) {
                let id = session.id.clone();
                state.open_session_tab(id);
            }
        }
        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.close_active_tab();
        }
        _ => {}
    }

    Action::None
}

fn handle_mouse_event(mouse: MouseEvent, state: &mut AppState) {
    match mouse.kind {
        MouseEventKind::ScrollDown => {
            if let Some(ref focused_id) = state.focused_session {
                if let Some(session) = state.snapshot.sessions.iter().find(|s| &s.id == focused_id)
                {
                    if let Some(pane) = session.active_pane() {
                        if let Some(preview) = state.previews.get_mut(&pane.id) {
                            preview.scroll_offset = preview.scroll_offset.saturating_add(3);
                        }
                    }
                }
            }
        }
        MouseEventKind::ScrollUp => {
            if let Some(ref focused_id) = state.focused_session {
                if let Some(session) = state.snapshot.sessions.iter().find(|s| &s.id == focused_id)
                {
                    if let Some(pane) = session.active_pane() {
                        if let Some(preview) = state.previews.get_mut(&pane.id) {
                            preview.scroll_offset = preview.scroll_offset.saturating_sub(3);
                            preview.at_bottom = false;
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

/// Main draw function
fn draw_ui(f: &mut ratatui::Frame, state: &mut AppState) {
    let size = f.area();
    let theme = state.config.theme();

    // Layout: tab bar | main content | status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Tab bar
            Constraint::Min(0),    // Main content
            Constraint::Length(1), // Status bar
        ])
        .split(size);

    // Tab bar
    tabs::draw_tabs(f, chunks[0], state, &theme);

    // Main content area
    match state.tabs.get(state.active_tab) {
        Some(Tab::Overview) | None => {
            draw_grid_view(f, chunks[1], state, &theme);
        }
        Some(Tab::Session(session_id)) => {
            let session_id = session_id.clone();
            draw_detail_view(f, chunks[1], state, &session_id, &theme);
        }
    }

    // Status bar
    status::draw_status(f, chunks[2], state, &theme);

    // Search overlay
    if state.search_active {
        draw_search_overlay(f, size, state, &theme);
    }

    // Command palette overlay
    if state.palette_open {
        draw_palette_overlay(f, size, state, &theme);
    }

    // Toast notification
    if let Some(ref msg) = state.toast_message {
        draw_toast(f, size, msg, &theme);
    }
}

fn draw_grid_view(
    f: &mut ratatui::Frame,
    area: Rect,
    state: &mut AppState,
    theme: &tmuxpulse::config::theme::ThemeConfig,
) {
    let sessions = state.visible_sessions();

    if sessions.is_empty() {
        let empty_msg = if state.snapshot.sessions.is_empty() {
            "No tmux sessions found. Start one with: tmux new -s mysession"
        } else {
            "All sessions filtered out. Press Esc to clear search."
        };
        let paragraph = Paragraph::new(empty_msg)
            .style(Style::default().fg(theme.fg_muted.to_ratatui()))
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
        return;
    }

    // Calculate grid layout
    let (cols, _rows) = layout::calculate_grid(
        area.width,
        area.height,
        sessions.len(),
        state.config.ui.card_min_width,
        state.config.ui.card_min_height,
    );

    let col_width = area.width / cols.max(1) as u16;

    // Draw session cards in grid
    for (i, session) in sessions.iter().enumerate() {
        let row = i / cols;
        let col = i % cols;

        let card_area = Rect {
            x: area.x + col as u16 * col_width,
            y: area.y + row as u16 * state.config.ui.card_min_height,
            width: col_width,
            height: state.config.ui.card_min_height,
        };

        // Clamp to available area
        if card_area.y + card_area.height > area.y + area.height {
            break;
        }

        let is_cursor = i == state.cursor_index;
        let is_focused = state.focused_session.as_ref() == Some(&session.id);
        let card_state = state
            .card_states
            .get(&session.id)
            .copied()
            .unwrap_or(CardState::Normal);

        let pane_content = session
            .active_pane()
            .and_then(|p| state.previews.get(&p.id))
            .map(|p| p.content.as_str())
            .unwrap_or("");

        let is_pulsing = session
            .active_pane()
            .and_then(|p| state.previews.get(&p.id))
            .map(|p| p.is_pulsing())
            .unwrap_or(false);

        let ctx = cards::CardRenderContext {
            session,
            pane_content,
            is_cursor,
            is_focused,
            is_pulsing,
            card_state,
        };
        cards::draw_session_card(f, card_area, &ctx, theme);
    }
}

fn draw_detail_view(
    f: &mut ratatui::Frame,
    area: Rect,
    state: &mut AppState,
    session_id: &SessionId,
    theme: &tmuxpulse::config::theme::ThemeConfig,
) {
    let session = state.snapshot.sessions.iter().find(|s| &s.id == session_id);
    let session = match session {
        Some(s) => s,
        None => {
            let msg = Paragraph::new("Session not found")
                .style(Style::default().fg(theme.error.to_ratatui()));
            f.render_widget(msg, area);
            return;
        }
    };

    let pane_content = session
        .active_pane()
        .and_then(|p| state.previews.get(&p.id))
        .map(|p| p.content.as_str())
        .unwrap_or("");

    let scroll_offset = session
        .active_pane()
        .and_then(|p| state.previews.get(&p.id))
        .map(|p| p.scroll_offset)
        .unwrap_or(0);

    // Header with session info
    let header = format!(
        " {} | {} | {} ",
        session.name,
        session
            .active_window()
            .map(|w| w.name.as_str())
            .unwrap_or("?"),
        session
            .active_pane()
            .map(|p| p.current_command.as_str())
            .unwrap_or("?"),
    );

    let border_color = theme.border_focused.to_ratatui();

    let block = Block::default()
        .title(header)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let content_lines: Vec<Line> = pane_content
        .lines()
        .map(|l| Line::from(l.to_string()))
        .collect();

    let total_lines = content_lines.len() as u16;
    let view_height = area.height.saturating_sub(2); // borders
    let effective_scroll = if scroll_offset == u16::MAX {
        total_lines.saturating_sub(view_height)
    } else {
        scroll_offset.min(total_lines.saturating_sub(view_height))
    };

    let paragraph = Paragraph::new(content_lines)
        .block(block)
        .scroll((effective_scroll, 0));

    f.render_widget(paragraph, area);
}

fn draw_search_overlay(
    f: &mut ratatui::Frame,
    area: Rect,
    state: &AppState,
    theme: &tmuxpulse::config::theme::ThemeConfig,
) {
    let search_area = Rect {
        x: area.x + 1,
        y: area.y + area.height.saturating_sub(3),
        width: area.width.saturating_sub(2).min(60),
        height: 1,
    };

    let search_text = format!("/ {}", state.search_query);
    let search_widget = Paragraph::new(search_text).style(
        Style::default()
            .fg(theme.fg_primary.to_ratatui())
            .bg(theme.status_bg.to_ratatui()),
    );

    f.render_widget(Clear, search_area);
    f.render_widget(search_widget, search_area);
}

fn draw_palette_overlay(
    f: &mut ratatui::Frame,
    area: Rect,
    state: &AppState,
    theme: &tmuxpulse::config::theme::ThemeConfig,
) {
    let actions = state.palette_actions();
    let palette_height = (actions.len() as u16 + 2).min(area.height.saturating_sub(4));
    let palette_width = 50u16.min(area.width.saturating_sub(4));

    let palette_area = Rect {
        x: (area.width.saturating_sub(palette_width)) / 2,
        y: (area.height.saturating_sub(palette_height)) / 3,
        width: palette_width,
        height: palette_height,
    };

    let block = Block::default()
        .title(" Command Palette ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent.to_ratatui()));

    let items: Vec<Line> = actions
        .iter()
        .enumerate()
        .map(|(i, action)| {
            let style = if i == state.palette_index {
                Style::default()
                    .fg(theme.fg_primary.to_ratatui())
                    .bg(theme.accent.to_ratatui())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg_secondary.to_ratatui())
            };
            Line::from(Span::styled(
                format!("  {}  {}", action.name, action.description),
                style,
            ))
        })
        .collect();

    let paragraph = Paragraph::new(items).block(block);

    f.render_widget(Clear, palette_area);
    f.render_widget(paragraph, palette_area);
}

fn draw_toast(
    f: &mut ratatui::Frame,
    area: Rect,
    message: &str,
    theme: &tmuxpulse::config::theme::ThemeConfig,
) {
    let toast_width = (message.len() as u16 + 4).min(area.width.saturating_sub(4));
    let toast_area = Rect {
        x: (area.width.saturating_sub(toast_width)) / 2,
        y: area.height / 2,
        width: toast_width,
        height: 1,
    };

    let toast = Paragraph::new(format!(" {} ", message)).style(
        Style::default()
            .fg(theme.fg_primary.to_ratatui())
            .bg(theme.accent.to_ratatui())
            .add_modifier(Modifier::BOLD),
    );

    f.render_widget(Clear, toast_area);
    f.render_widget(toast, toast_area);
}
