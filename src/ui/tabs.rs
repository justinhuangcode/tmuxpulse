use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use tmuxpulse::config::theme::ThemeConfig;
use tmuxpulse::state::{AppState, Tab};

pub fn draw_tabs(f: &mut Frame, area: Rect, state: &AppState, theme: &ThemeConfig) {
    let mut spans = Vec::new();

    for (i, tab) in state.tabs.iter().enumerate() {
        let label = match tab {
            Tab::Overview => " Overview ".to_string(),
            Tab::Session(id) => {
                let name = state
                    .snapshot
                    .sessions
                    .iter()
                    .find(|s| &s.id == id)
                    .map(|s| s.name.as_str())
                    .unwrap_or("?");
                format!(" {} ", name)
            }
        };

        let style = if i == state.active_tab {
            Style::default()
                .fg(theme.tab_active.to_ratatui())
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.tab_inactive.to_ratatui())
        };

        spans.push(Span::styled(label, style));

        if i + 1 < state.tabs.len() {
            spans.push(Span::styled(
                " | ",
                Style::default().fg(theme.fg_muted.to_ratatui()),
            ));
        }
    }

    let tabs_line = Paragraph::new(Line::from(spans));
    f.render_widget(tabs_line, area);
}
