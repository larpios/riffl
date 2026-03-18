/// Theme and color scheme management
use ratatui::style::{Color, Modifier, Style};

/// Available built-in themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeKind {
    /// Catppuccin Latte — warm, creamy light theme
    CatppuccinLatte,
    /// Catppuccin Frappé — cool medium-dark
    CatppuccinFrappe,
    /// Catppuccin Macchiato — deeper dark
    CatppuccinMacchiato,
    /// Catppuccin Mocha — darkest Catppuccin (default)
    #[default]
    CatppuccinMocha,
    /// Nord — cool arctic palette
    Nord,
    /// Dark — classic terminal dark (raw colors)
    Dark,
}

impl ThemeKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::CatppuccinLatte => "latte",
            Self::CatppuccinFrappe => "frappe",
            Self::CatppuccinMacchiato => "macchiato",
            Self::CatppuccinMocha => "mocha",
            Self::Nord => "nord",
            Self::Dark => "dark",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "latte" | "catppuccin-latte" | "cappuccino" => Some(Self::CatppuccinLatte),
            "frappe" | "catppuccin-frappe" => Some(Self::CatppuccinFrappe),
            "macchiato" | "catppuccin-macchiato" => Some(Self::CatppuccinMacchiato),
            "mocha" | "catppuccin" | "catppuccin-mocha" => Some(Self::CatppuccinMocha),
            "nord" => Some(Self::Nord),
            "dark" | "default" => Some(Self::Dark),
            _ => None,
        }
    }

    /// All available theme names, for display in help / error messages.
    pub fn all_names() -> &'static [&'static str] {
        &["latte", "frappe", "macchiato", "mocha", "nord", "dark"]
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
}

impl Theme {
    pub fn from_kind(kind: ThemeKind) -> Self {
        match kind {
            ThemeKind::CatppuccinLatte => Self::catppuccin_latte(),
            ThemeKind::CatppuccinFrappe => Self::catppuccin_frappe(),
            ThemeKind::CatppuccinMacchiato => Self::catppuccin_macchiato(),
            ThemeKind::CatppuccinMocha => Self::catppuccin_mocha(),
            ThemeKind::Nord => Self::nord(),
            ThemeKind::Dark => Self::dark(),
        }
    }

    // ── Dark (default) ──────────────────────────────────────────────────────
    pub fn dark() -> Self {
        Self {
            bg: Color::Reset,
            bg_surface: Color::Reset,
            bg_highlight: Color::Yellow,
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
        }
    }

    // ── Catppuccin Latte ────────────────────────────────────────────────────
    pub fn catppuccin_latte() -> Self {
        // https://github.com/catppuccin/catppuccin — Latte palette
        let base = Color::Rgb(239, 241, 245); // #eff1f5
        let mantle = Color::Rgb(230, 233, 239); // #e6e9ef
        let surface0 = Color::Rgb(204, 208, 218); // #ccd0da
        let surface1 = Color::Rgb(188, 192, 204); // #bcc0cc
        let overlay1 = Color::Rgb(140, 143, 161); // #8c8fa1
        let text = Color::Rgb(76, 79, 105); // #4c4f69
        let subtext0 = Color::Rgb(108, 111, 133); // #6c6f85
        let blue = Color::Rgb(30, 102, 245); // #1e66f5
        let lavender = Color::Rgb(114, 135, 253); // #7287fd
        let green = Color::Rgb(64, 160, 43); // #40a02b
        let yellow = Color::Rgb(223, 142, 29); // #df8e1d
        let peach = Color::Rgb(254, 100, 11); // #fe640b
        let red = Color::Rgb(210, 15, 57); // #d20f39
        let mauve = Color::Rgb(136, 57, 239); // #8839ef
        let teal = Color::Rgb(23, 146, 153); // #179299
        let crust = Color::Rgb(220, 224, 232); // #dce0e8

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
            cursor_visual_bg: surface1,
            cursor_fg: crust,

            status_success: green,
            status_warning: yellow,
            status_error: red,
            status_info: teal,
        }
    }

    // ── Catppuccin Frappé ───────────────────────────────────────────────────
    pub fn catppuccin_frappe() -> Self {
        // https://github.com/catppuccin/catppuccin — Frappé palette
        let base = Color::Rgb(48, 52, 70); // #303446
        let mantle = Color::Rgb(41, 44, 60); // #292c3c
        let surface0 = Color::Rgb(65, 69, 89); // #414559
        let surface1 = Color::Rgb(81, 87, 109); // #51576d
        let overlay1 = Color::Rgb(131, 139, 167); // #838ba7
        let text = Color::Rgb(198, 208, 245); // #c6d0f5
        let subtext0 = Color::Rgb(165, 173, 206); // #a5adce
        let blue = Color::Rgb(140, 170, 238); // #8caaee
        let lavender = Color::Rgb(186, 187, 241); // #babbf1
        let green = Color::Rgb(166, 209, 137); // #a6d189
        let yellow = Color::Rgb(229, 200, 144); // #e5c890
        let peach = Color::Rgb(239, 159, 118); // #ef9f76
        let red = Color::Rgb(231, 130, 132); // #e78284
        let mauve = Color::Rgb(202, 158, 230); // #ca9ee6
        let teal = Color::Rgb(129, 200, 190); // #81c8be
        let crust = Color::Rgb(35, 38, 52); // #232634

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
            cursor_visual_bg: surface1,
            cursor_fg: crust,

            status_success: green,
            status_warning: yellow,
            status_error: red,
            status_info: teal,
        }
    }

    // ── Catppuccin Macchiato ─────────────────────────────────────────────────
    pub fn catppuccin_macchiato() -> Self {
        // https://github.com/catppuccin/catppuccin — Macchiato palette
        let base = Color::Rgb(36, 39, 58); // #24273a
        let mantle = Color::Rgb(30, 32, 48); // #1e2030
        let surface0 = Color::Rgb(54, 58, 79); // #363a4f
        let surface1 = Color::Rgb(73, 77, 100); // #494d64
        let overlay1 = Color::Rgb(128, 135, 162); // #8087a2
        let text = Color::Rgb(202, 211, 245); // #cad3f5
        let subtext0 = Color::Rgb(165, 173, 203); // #a5adcb
        let blue = Color::Rgb(138, 173, 244); // #8aadf4
        let lavender = Color::Rgb(183, 189, 248); // #b7bdf8
        let green = Color::Rgb(166, 218, 149); // #a6da95
        let yellow = Color::Rgb(238, 212, 159); // #eed49f
        let peach = Color::Rgb(245, 169, 127); // #f5a97f
        let red = Color::Rgb(237, 135, 150); // #ed8796
        let mauve = Color::Rgb(198, 160, 246); // #c6a0f6
        let teal = Color::Rgb(139, 213, 202); // #8bd5ca
        let crust = Color::Rgb(24, 25, 38); // #181926

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
            cursor_visual_bg: surface1,
            cursor_fg: crust,

            status_success: green,
            status_warning: yellow,
            status_error: red,
            status_info: teal,
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
            cursor_visual_bg: Color::Rgb(69, 71, 90), // surface1
            cursor_fg: crust,

            status_success: green,
            status_warning: yellow,
            status_error: red,
            status_info: teal,
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
            cursor_visual_bg: polar2,
            cursor_fg: polar0,

            status_success: aurora_green,
            status_warning: aurora_yellow,
            status_error: aurora_red,
            status_info: frost0,
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

    pub fn visual_selection_style(&self) -> Style {
        Style::default().fg(self.text).bg(self.cursor_visual_bg)
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
        Self::catppuccin_mocha()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_default_is_mocha() {
        assert_eq!(Theme::default(), Theme::catppuccin_mocha());
    }

    #[test]
    fn test_from_kind_roundtrip() {
        for kind in [
            ThemeKind::CatppuccinLatte,
            ThemeKind::CatppuccinFrappe,
            ThemeKind::CatppuccinMacchiato,
            ThemeKind::CatppuccinMocha,
            ThemeKind::Nord,
            ThemeKind::Dark,
        ] {
            let t = Theme::from_kind(kind);
            assert_eq!(t, Theme::from_kind(kind));
        }
    }

    #[test]
    fn test_theme_kind_from_str() {
        assert_eq!(ThemeKind::from_str("latte"), Some(ThemeKind::CatppuccinLatte));
        assert_eq!(ThemeKind::from_str("cappuccino"), Some(ThemeKind::CatppuccinLatte));
        assert_eq!(ThemeKind::from_str("frappe"), Some(ThemeKind::CatppuccinFrappe));
        assert_eq!(ThemeKind::from_str("macchiato"), Some(ThemeKind::CatppuccinMacchiato));
        assert_eq!(ThemeKind::from_str("mocha"), Some(ThemeKind::CatppuccinMocha));
        assert_eq!(ThemeKind::from_str("catppuccin-mocha"), Some(ThemeKind::CatppuccinMocha));
        assert_eq!(ThemeKind::from_str("nord"), Some(ThemeKind::Nord));
        assert_eq!(ThemeKind::from_str("dark"), Some(ThemeKind::Dark));
        assert_eq!(ThemeKind::from_str("unknown"), None);
    }

    #[test]
    fn test_theme_kind_name() {
        assert_eq!(ThemeKind::CatppuccinLatte.name(), "latte");
        assert_eq!(ThemeKind::CatppuccinFrappe.name(), "frappe");
        assert_eq!(ThemeKind::CatppuccinMacchiato.name(), "macchiato");
        assert_eq!(ThemeKind::CatppuccinMocha.name(), "mocha");
        assert_eq!(ThemeKind::Nord.name(), "nord");
        assert_eq!(ThemeKind::Dark.name(), "dark");
    }

    #[test]
    fn test_theme_kind_all_names() {
        let names = ThemeKind::all_names();
        assert!(names.contains(&"latte"));
        assert!(names.contains(&"frappe"));
        assert!(names.contains(&"macchiato"));
        assert!(names.contains(&"mocha"));
        assert!(names.contains(&"nord"));
        assert!(names.contains(&"dark"));
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
    fn test_latte_theme_has_rgb_colors() {
        let t = Theme::catppuccin_latte();
        assert!(matches!(t.bg, Color::Rgb(_, _, _)));
        assert!(matches!(t.primary, Color::Rgb(_, _, _)));
        assert!(matches!(t.text, Color::Rgb(_, _, _)));
    }

    #[test]
    fn test_frappe_theme_has_rgb_colors() {
        let t = Theme::catppuccin_frappe();
        assert!(matches!(t.bg, Color::Rgb(_, _, _)));
        assert!(matches!(t.primary, Color::Rgb(_, _, _)));
    }

    #[test]
    fn test_macchiato_theme_has_rgb_colors() {
        let t = Theme::catppuccin_macchiato();
        assert!(matches!(t.bg, Color::Rgb(_, _, _)));
        assert!(matches!(t.primary, Color::Rgb(_, _, _)));
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
    fn test_style_methods_use_theme_colors() {
        let t = Theme::catppuccin_mocha();
        assert_eq!(t.header_style().fg, Some(t.text));
        assert_eq!(t.footer_style().bg, Some(t.bg_footer));
        assert_eq!(t.border_style().fg, Some(t.border));
        assert_eq!(t.highlight_style().bg, Some(t.cursor_normal_bg));
        assert_eq!(t.text_style().fg, Some(t.text));
        assert_eq!(t.dimmed_style().fg, Some(t.text_dimmed));
    }
}
