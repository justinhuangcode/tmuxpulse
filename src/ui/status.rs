use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use tmuxpulse::config::theme::ThemeConfig;
use tmuxpulse::state::AppState;

pub fn draw_status(f: &mut Frame, area: Rect, state: &AppState, theme: &ThemeConfig) {
    let session_count = state.snapshot.session_count();
    let pane_count = state.snapshot.pane_count();
    let stale_count = state
        .snapshot
        .sessions
        .iter()
        .filter(|s| s.is_stale(state.config.general.stale_threshold_secs))
        .count();

    let left = format!(
        " {} sessions | {} panes{}",
        session_count,
        pane_count,
        if stale_count > 0 {
            format!(" | {} stale", stale_count)
        } else {
            String::new()
        },
    );

    let right = if let Some(ref err) = state.last_error {
        format!(" ERR: {} ", err)
    } else {
        let shortcuts = if state.focused_session.is_some() {
            "Esc:unfocus Up/Down:scroll"
        } else {
            "/:search Ctrl+P:palette q:quit"
        };
        format!(" {} ", shortcuts)
    };

    let left_style = Style::default()
        .fg(theme.fg_secondary.to_ratatui())
        .bg(theme.status_bg.to_ratatui());

    let right_style = if state.last_error.is_some() {
        Style::default()
            .fg(theme.error.to_ratatui())
            .bg(theme.status_bg.to_ratatui())
    } else {
        Style::default()
            .fg(theme.fg_muted.to_ratatui())
            .bg(theme.status_bg.to_ratatui())
    };

    // Fill the status bar background
    let bg_fill = " ".repeat(area.width as usize);
    let bg = Paragraph::new(bg_fill).style(Style::default().bg(theme.status_bg.to_ratatui()));
    f.render_widget(bg, area);

    // Left-aligned status
    let left_area = Rect {
        width: (left.len() as u16).min(area.width / 2),
        ..area
    };
    f.render_widget(Paragraph::new(left).style(left_style), left_area);

    // Right-aligned shortcuts
    let right_len = right.len() as u16;
    let right_area = Rect {
        x: area.x + area.width.saturating_sub(right_len),
        width: right_len.min(area.width / 2),
        ..area
    };
    f.render_widget(Paragraph::new(right).style(right_style), right_area);
}
