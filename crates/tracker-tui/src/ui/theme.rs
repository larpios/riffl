/// Theme and color scheme management
use ratatui::style::{Color, Modifier, Style};

/// Available built-in themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeKind {
    #[default]
    Dark,
    CatppuccinMocha,
    Nord,
}

impl ThemeKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::CatppuccinMocha => "mocha",
            Self::Nord => "nord",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "dark" | "default" => Some(Self::Dark),
            "mocha" | "catppuccin" | "catppuccin-mocha" => Some(Self::CatppuccinMocha),
            "nord" => Some(Self::Nord),
            _ => None,
        }
    }
}

/// Color palette for the application theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    // Backgrounds
    /// Main terminal background
    pub bg: Color,
    /// Panel / container background (slightly elevated)
    pub bg_surface: Color,
    /// Highlighted row / selected item background
    pub bg_highlight: Color,
    /// Header area background
    pub bg_header: Color,
    /// Footer area background
    pub bg_footer: Color,

    // Primary accents
    pub primary: Color,
    pub secondary: Color,

    // Borders
    pub border: Color,
    pub border_focused: Color,

    // Text
    pub text: Color,
    pub text_secondary: Color,
    pub text_dimmed: Color,

    // Cursor / selection backgrounds
    pub cursor_normal_bg: Color,
    pub cursor_insert_bg: Color,
    pub cursor_visual_bg: Color,
    pub cursor_fg: Color,

    // Status
    pub status_success: Color,
    pub status_warning: Color,
    pub status_error: Color,
    pub status_info: Color,

    // Pattern grid cell sub-column colors
    pub note_color: Color,
    pub inst_color: Color,
    pub vol_color: Color,
    pub eff_color: Color,
}

impl Theme {
    pub fn from_kind(kind: ThemeKind) -> Self {
        match kind {
            ThemeKind::Dark => Self::dark(),
            ThemeKind::CatppuccinMocha => Self::catppuccin_mocha(),
            ThemeKind::Nord => Self::nord(),
        }
    }

    // ── Dark (default) ──────────────────────────────────────────────────────
    pub fn dark() -> Self {
        Self {
            bg: Color::Reset,
            bg_surface: Color::Reset,
            bg_highlight: Color::DarkGray,
            bg_header: Color::Reset,
            bg_footer: Color::DarkGray,

            primary: Color::Cyan,
            secondary: Color::Blue,

            border: Color::Cyan,
            border_focused: Color::Yellow,

            text: Color::White,
            text_secondary: Color::Gray,
            text_dimmed: Color::DarkGray,

            cursor_normal_bg: Color::Yellow,
            cursor_insert_bg: Color::LightMagenta,
            cursor_visual_bg: Color::Blue,
            cursor_fg: Color::Black,

            status_success: Color::Green,
            status_warning: Color::Yellow,
            status_error: Color::Red,
            status_info: Color::Cyan,

            note_color: Color::Cyan,
            inst_color: Color::Yellow,
            vol_color: Color::Magenta,
            eff_color: Color::LightYellow,
        }
    }

    // ── Catppuccin Mocha ────────────────────────────────────────────────────
    pub fn catppuccin_mocha() -> Self {
        // https://github.com/catppuccin/catppuccin
        let base = Color::Rgb(30, 30, 46); // #1e1e2e
        let mantle = Color::Rgb(24, 24, 37); // #181825
        let surface0 = Color::Rgb(49, 50, 68); // #313244
        let surface1 = Color::Rgb(69, 71, 90); // #45475a
        let overlay1 = Color::Rgb(127, 132, 156); // #7f849c
        let text = Color::Rgb(205, 214, 244); // #cdd6f4
        let subtext0 = Color::Rgb(166, 173, 200); // #a6adc8
        let blue = Color::Rgb(137, 180, 250); // #89b4fa
        let lavender = Color::Rgb(180, 190, 254); // #b4befe
        let green = Color::Rgb(166, 227, 161); // #a6e3a1
        let yellow = Color::Rgb(249, 226, 175); // #f9e2af
        let peach = Color::Rgb(250, 179, 135); // #fab387
        let red = Color::Rgb(243, 139, 168); // #f38ba8
        let mauve = Color::Rgb(203, 166, 247); // #cba6f7
        let teal = Color::Rgb(148, 226, 213); // #94e2d5
        let crust = Color::Rgb(17, 17, 27); // #11111b

        Self {
            bg: base,
            bg_surface: mantle,
            bg_highlight: surface0,
            bg_header: mantle,
            bg_footer: mantle,

            primary: blue,
            secondary: lavender,

            border: surface1,
            border_focused: blue,

            text,
            text_secondary: subtext0,
            text_dimmed: overlay1,

            cursor_normal_bg: peach,
            cursor_insert_bg: mauve,
            cursor_visual_bg: surface1, // #45475a — selection bg (nvim Catppuccin uses surface1 for Visual)
            cursor_fg: crust,

            status_success: green,
            status_warning: yellow,
            status_error: red,
            status_info: teal,

            note_color: blue,
            inst_color: yellow,
            vol_color: mauve,
            eff_color: peach,
        }
    }

    // ── Nord ────────────────────────────────────────────────────────────────
    pub fn nord() -> Self {
        let polar0 = Color::Rgb(46, 52, 64); // #2e3440
        let polar1 = Color::Rgb(59, 66, 82); // #3b4252
        let polar2 = Color::Rgb(67, 76, 94); // #434c5e
        let polar3 = Color::Rgb(76, 86, 106); // #4c566a
        let snow0 = Color::Rgb(216, 222, 233); // #d8dee9
        let _snow1 = Color::Rgb(229, 233, 240); // #e5e9f0
        let frost0 = Color::Rgb(143, 188, 187); // #8fbcbb
        let frost1 = Color::Rgb(136, 192, 208); // #88c0d0
        let frost3 = Color::Rgb(129, 161, 193); // #81a1c1
        let aurora_red = Color::Rgb(191, 97, 106); // #bf616a
        let aurora_orange = Color::Rgb(208, 135, 112); // #d08770
        let aurora_yellow = Color::Rgb(235, 203, 139); // #ebcb8b
        let aurora_green = Color::Rgb(163, 190, 140); // #a3be8c

        Self {
            bg: polar0,
            bg_surface: polar1,
            bg_highlight: polar2,
            bg_header: polar1,
            bg_footer: polar1,

            primary: frost1,
            secondary: frost3,

            border: polar3,
            border_focused: frost1,

            text: snow0,
            text_secondary: Color::Rgb(180, 186, 198),
            text_dimmed: polar3,

            cursor_normal_bg: aurora_yellow,
            cursor_insert_bg: aurora_orange,
            cursor_visual_bg: frost3, // #81a1c1 — clearly distinct from polar2 bg_highlight
            cursor_fg: polar0,

            status_success: aurora_green,
            status_warning: aurora_yellow,
            status_error: aurora_red,
            status_info: frost0,

            note_color: frost1,
            inst_color: aurora_yellow,
            vol_color: aurora_orange,
            eff_color: aurora_green,
        }
    }

    // ── Style helpers ────────────────────────────────────────────────────────

    pub fn header_style(&self) -> Style {
        Style::default()
            .fg(self.text)
            .bg(self.bg_header)
            .add_modifier(Modifier::BOLD)
    }

    pub fn footer_style(&self) -> Style {
        Style::default().fg(self.text).bg(self.bg_footer)
    }

    pub fn border_color(&self) -> Color {
        self.border
    }

    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border)
    }

    pub fn focused_border_style(&self) -> Style {
        Style::default().fg(self.border_focused)
    }

    pub fn highlight_style(&self) -> Style {
        Style::default()
            .fg(self.cursor_fg)
            .bg(self.cursor_normal_bg)
    }

    pub fn insert_cursor_style(&self) -> Style {
        Style::default()
            .fg(self.cursor_fg)
            .bg(self.cursor_insert_bg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn insert_inactive_style(&self) -> Style {
        Style::default()
            .fg(self.text_secondary)
            .bg(self.bg_highlight)
    }

    /// Style for non-cursor sub-columns when in Normal mode cursor.
    pub fn normal_inactive_style(&self) -> Style {
        Style::default()
            .fg(self.text_secondary)
            .bg(self.bg_highlight)
    }

    pub fn visual_selection_style(&self) -> Style {
        Style::default().fg(self.text).bg(self.cursor_visual_bg)
    }

    /// Style for the cursor cell while in Visual mode (distinct from the rest of the selection).
    /// Uses cursor_insert_bg (e.g. mauve in Catppuccin) so it stands out from both the
    /// surface1 selection bg and the normal/insert cursors, while staying theme-consistent.
    pub fn visual_cursor_style(&self) -> Style {
        Style::default()
            .fg(self.cursor_fg)
            .bg(self.cursor_insert_bg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn text_style(&self) -> Style {
        Style::default().fg(self.text).bg(self.bg)
    }

    pub fn dimmed_style(&self) -> Style {
        Style::default().fg(self.text_dimmed)
    }

    pub fn success_color(&self) -> Color {
        self.status_success
    }
    pub fn warning_color(&self) -> Color {
        self.status_warning
    }
    pub fn error_color(&self) -> Color {
        self.status_error
    }
    pub fn info_color(&self) -> Color {
        self.status_info
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_default_is_dark() {
        assert_eq!(Theme::default(), Theme::dark());
    }

    #[test]
    fn test_from_kind_roundtrip() {
        for kind in [ThemeKind::Dark, ThemeKind::CatppuccinMocha, ThemeKind::Nord] {
            let t = Theme::from_kind(kind);
            assert_eq!(t, Theme::from_kind(kind));
        }
    }

    #[test]
    fn test_theme_kind_from_str() {
        assert_eq!(ThemeKind::from_str("dark"), Some(ThemeKind::Dark));
        assert_eq!(
            ThemeKind::from_str("mocha"),
            Some(ThemeKind::CatppuccinMocha)
        );
        assert_eq!(
            ThemeKind::from_str("catppuccin-mocha"),
            Some(ThemeKind::CatppuccinMocha)
        );
        assert_eq!(ThemeKind::from_str("nord"), Some(ThemeKind::Nord));
        assert_eq!(ThemeKind::from_str("unknown"), None);
    }

    #[test]
    fn test_theme_kind_name() {
        assert_eq!(ThemeKind::Dark.name(), "dark");
        assert_eq!(ThemeKind::CatppuccinMocha.name(), "mocha");
        assert_eq!(ThemeKind::Nord.name(), "nord");
    }

    #[test]
    fn test_dark_theme_colors() {
        let t = Theme::dark();
        assert_eq!(t.primary, Color::Cyan);
        assert_eq!(t.text, Color::White);
        assert_eq!(t.status_success, Color::Green);
        assert_eq!(t.status_error, Color::Red);
        assert_eq!(t.cursor_normal_bg, Color::Yellow);
        assert_eq!(t.cursor_insert_bg, Color::LightMagenta);
    }

    #[test]
    fn test_mocha_theme_has_rgb_colors() {
        let t = Theme::catppuccin_mocha();
        assert!(matches!(t.bg, Color::Rgb(_, _, _)));
        assert!(matches!(t.primary, Color::Rgb(_, _, _)));
        assert!(matches!(t.text, Color::Rgb(_, _, _)));
    }

    #[test]
    fn test_nord_theme_has_rgb_colors() {
        let t = Theme::nord();
        assert!(matches!(t.bg, Color::Rgb(_, _, _)));
        assert!(matches!(t.primary, Color::Rgb(_, _, _)));
    }

    #[test]
    fn test_theme_has_cell_colors() {
        for t in [Theme::dark(), Theme::catppuccin_mocha(), Theme::nord()] {
            let _ = t.note_color;
            let _ = t.inst_color;
            let _ = t.vol_color;
            let _ = t.eff_color;
        }
    }

    #[test]
    fn test_dark_theme_cell_colors() {
        let t = Theme::dark();
        assert_eq!(t.inst_color, Color::Yellow);
        assert_eq!(t.vol_color, Color::Magenta);
    }

    #[test]
    fn test_style_methods_use_theme_colors() {
        let t = Theme::dark();
        assert_eq!(t.header_style().fg, Some(t.text));
        assert_eq!(t.footer_style().bg, Some(t.bg_footer));
        assert_eq!(t.border_style().fg, Some(t.border));
        assert_eq!(t.highlight_style().bg, Some(t.cursor_normal_bg));
        assert_eq!(t.text_style().fg, Some(t.text));
        assert_eq!(t.dimmed_style().fg, Some(t.text_dimmed));
    }
}
