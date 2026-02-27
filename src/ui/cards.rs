use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use tmuxpulse::config::theme::ThemeConfig;
use tmuxpulse::mux::Session;
use tmuxpulse::state::CardState;

/// Visual state passed to card rendering
pub struct CardRenderContext<'a> {
    pub session: &'a Session,
    pub pane_content: &'a str,
    pub is_cursor: bool,
    pub is_focused: bool,
    pub is_pulsing: bool,
    pub card_state: CardState,
}

pub fn draw_session_card(
    f: &mut Frame,
    area: Rect,
    ctx: &CardRenderContext<'_>,
    theme: &ThemeConfig,
) {
    let session = ctx.session;

    // Determine border color
    let border_color = if ctx.is_focused {
        theme.border_focused.to_ratatui()
    } else if ctx.is_cursor {
        theme.border_cursor.to_ratatui()
    } else if ctx.is_pulsing {
        theme.border_pulse.to_ratatui()
    } else if session.is_stale(3600) {
        theme.border_stale.to_ratatui()
    } else {
        theme.border_normal.to_ratatui()
    };

    // Build title
    let active_window = session
        .active_window()
        .map(|w| w.name.as_str())
        .unwrap_or("?");
    let active_cmd = session
        .active_pane()
        .map(|p| p.current_command.as_str())
        .unwrap_or("?");

    let title = format!(" {} | {} | {} ", session.name, active_window, active_cmd);

    // Status indicators
    let mut indicators = Vec::new();
    if session.attached {
        indicators.push("A");
    }
    if session.is_stale(3600) {
        indicators.push("S");
    }
    let indicator_str = if indicators.is_empty() {
        String::new()
    } else {
        format!(" [{}] ", indicators.join(""))
    };

    let border_style = Style::default().fg(border_color);
    let title_style = if ctx.is_focused {
        Style::default()
            .fg(border_color)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(border_color)
    };

    match ctx.card_state {
        CardState::Collapsed => {
            let collapsed_area = Rect {
                height: 1.min(area.height),
                ..area
            };
            let header = Paragraph::new(Line::from(vec![
                Span::styled(title, title_style),
                Span::styled(
                    indicator_str,
                    Style::default().fg(theme.fg_muted.to_ratatui()),
                ),
                Span::styled(" [+]", Style::default().fg(theme.fg_muted.to_ratatui())),
            ]));
            f.render_widget(header, collapsed_area);
        }
        CardState::Hidden => {}
        CardState::Normal | CardState::Maximized => {
            let block = Block::default()
                .title(Line::from(vec![
                    Span::styled(title, title_style),
                    Span::styled(
                        indicator_str,
                        Style::default().fg(theme.fg_muted.to_ratatui()),
                    ),
                ]))
                .borders(Borders::ALL)
                .border_style(border_style);

            let content_lines: Vec<Line> = if ctx.pane_content.is_empty() {
                vec![Line::from(Span::styled(
                    "  (no output)",
                    Style::default().fg(theme.fg_muted.to_ratatui()),
                ))]
            } else {
                let available_height = area.height.saturating_sub(2) as usize;
                let lines: Vec<&str> = ctx.pane_content.lines().collect();
                let start = lines.len().saturating_sub(available_height);
                lines[start..]
                    .iter()
                    .map(|l| Line::from(l.to_string()))
                    .collect()
            };

            let paragraph = Paragraph::new(content_lines)
                .block(block)
                .wrap(Wrap { trim: false });

            f.render_widget(paragraph, area);
        }
    }
}
