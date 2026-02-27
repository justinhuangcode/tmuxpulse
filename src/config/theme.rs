use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub name: String,
    pub border_normal: ThemeColor,
    pub border_focused: ThemeColor,
    pub border_cursor: ThemeColor,
    pub border_hover: ThemeColor,
    pub border_pulse: ThemeColor,
    pub border_stale: ThemeColor,
    pub border_dead: ThemeColor,
    pub bg_primary: ThemeColor,
    pub bg_secondary: ThemeColor,
    pub fg_primary: ThemeColor,
    pub fg_secondary: ThemeColor,
    pub fg_muted: ThemeColor,
    pub accent: ThemeColor,
    pub warning: ThemeColor,
    pub error: ThemeColor,
    pub success: ThemeColor,
    pub tab_active: ThemeColor,
    pub tab_inactive: ThemeColor,
    pub status_bg: ThemeColor,
    pub search_highlight: ThemeColor,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ThemeColor {
    Indexed(u8),
    Rgb(u8, u8, u8),
}

impl ThemeColor {
    pub fn to_ratatui(self) -> Color {
        match self {
            ThemeColor::Indexed(i) => Color::Indexed(i),
            ThemeColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
        }
    }
}

impl ThemeConfig {
    pub fn by_name(name: &str) -> Self {
        match name {
            "catppuccin-mocha" => Self::catppuccin_mocha(),
            "dracula" => Self::dracula(),
            "nord" => Self::nord(),
            _ => Self::default_theme(),
        }
    }

    pub fn default_theme() -> Self {
        Self {
            name: "default".to_string(),
            border_normal: ThemeColor::Indexed(245),
            border_focused: ThemeColor::Indexed(212),
            border_cursor: ThemeColor::Indexed(111),
            border_hover: ThemeColor::Indexed(143),
            border_pulse: ThemeColor::Indexed(213),
            border_stale: ThemeColor::Indexed(95),
            border_dead: ThemeColor::Indexed(203),
            bg_primary: ThemeColor::Indexed(0),
            bg_secondary: ThemeColor::Indexed(235),
            fg_primary: ThemeColor::Indexed(255),
            fg_secondary: ThemeColor::Indexed(250),
            fg_muted: ThemeColor::Indexed(245),
            accent: ThemeColor::Indexed(75),
            warning: ThemeColor::Indexed(214),
            error: ThemeColor::Indexed(203),
            success: ThemeColor::Indexed(36),
            tab_active: ThemeColor::Indexed(212),
            tab_inactive: ThemeColor::Indexed(245),
            status_bg: ThemeColor::Indexed(236),
            search_highlight: ThemeColor::Indexed(226),
        }
    }

    pub fn catppuccin_mocha() -> Self {
        Self {
            name: "catppuccin-mocha".to_string(),
            border_normal: ThemeColor::Rgb(88, 91, 112),
            border_focused: ThemeColor::Rgb(245, 194, 231),
            border_cursor: ThemeColor::Rgb(137, 180, 250),
            border_hover: ThemeColor::Rgb(249, 226, 175),
            border_pulse: ThemeColor::Rgb(243, 139, 168),
            border_stale: ThemeColor::Rgb(147, 153, 178),
            border_dead: ThemeColor::Rgb(243, 139, 168),
            bg_primary: ThemeColor::Rgb(30, 30, 46),
            bg_secondary: ThemeColor::Rgb(24, 24, 37),
            fg_primary: ThemeColor::Rgb(205, 214, 244),
            fg_secondary: ThemeColor::Rgb(186, 194, 222),
            fg_muted: ThemeColor::Rgb(147, 153, 178),
            accent: ThemeColor::Rgb(137, 180, 250),
            warning: ThemeColor::Rgb(249, 226, 175),
            error: ThemeColor::Rgb(243, 139, 168),
            success: ThemeColor::Rgb(166, 227, 161),
            tab_active: ThemeColor::Rgb(245, 194, 231),
            tab_inactive: ThemeColor::Rgb(88, 91, 112),
            status_bg: ThemeColor::Rgb(24, 24, 37),
            search_highlight: ThemeColor::Rgb(249, 226, 175),
        }
    }

    pub fn dracula() -> Self {
        Self {
            name: "dracula".to_string(),
            border_normal: ThemeColor::Rgb(68, 71, 90),
            border_focused: ThemeColor::Rgb(255, 121, 198),
            border_cursor: ThemeColor::Rgb(139, 233, 253),
            border_hover: ThemeColor::Rgb(241, 250, 140),
            border_pulse: ThemeColor::Rgb(255, 85, 85),
            border_stale: ThemeColor::Rgb(98, 114, 164),
            border_dead: ThemeColor::Rgb(255, 85, 85),
            bg_primary: ThemeColor::Rgb(40, 42, 54),
            bg_secondary: ThemeColor::Rgb(33, 34, 44),
            fg_primary: ThemeColor::Rgb(248, 248, 242),
            fg_secondary: ThemeColor::Rgb(189, 147, 249),
            fg_muted: ThemeColor::Rgb(98, 114, 164),
            accent: ThemeColor::Rgb(139, 233, 253),
            warning: ThemeColor::Rgb(241, 250, 140),
            error: ThemeColor::Rgb(255, 85, 85),
            success: ThemeColor::Rgb(80, 250, 123),
            tab_active: ThemeColor::Rgb(255, 121, 198),
            tab_inactive: ThemeColor::Rgb(68, 71, 90),
            status_bg: ThemeColor::Rgb(33, 34, 44),
            search_highlight: ThemeColor::Rgb(241, 250, 140),
        }
    }

    pub fn nord() -> Self {
        Self {
            name: "nord".to_string(),
            border_normal: ThemeColor::Rgb(76, 86, 106),
            border_focused: ThemeColor::Rgb(136, 192, 208),
            border_cursor: ThemeColor::Rgb(129, 161, 193),
            border_hover: ThemeColor::Rgb(235, 203, 139),
            border_pulse: ThemeColor::Rgb(191, 97, 106),
            border_stale: ThemeColor::Rgb(76, 86, 106),
            border_dead: ThemeColor::Rgb(191, 97, 106),
            bg_primary: ThemeColor::Rgb(46, 52, 64),
            bg_secondary: ThemeColor::Rgb(59, 66, 82),
            fg_primary: ThemeColor::Rgb(236, 239, 244),
            fg_secondary: ThemeColor::Rgb(229, 233, 240),
            fg_muted: ThemeColor::Rgb(76, 86, 106),
            accent: ThemeColor::Rgb(136, 192, 208),
            warning: ThemeColor::Rgb(235, 203, 139),
            error: ThemeColor::Rgb(191, 97, 106),
            success: ThemeColor::Rgb(163, 190, 140),
            tab_active: ThemeColor::Rgb(136, 192, 208),
            tab_inactive: ThemeColor::Rgb(76, 86, 106),
            status_bg: ThemeColor::Rgb(59, 66, 82),
            search_highlight: ThemeColor::Rgb(235, 203, 139),
        }
    }
}
