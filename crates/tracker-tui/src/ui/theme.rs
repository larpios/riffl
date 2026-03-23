/// Theme and color scheme management
use ratatui::style::{Color, Modifier, Style};
use std::path::Path;

/// Available built-in themes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ThemeKind {
    #[default]
    Dark,
    CatppuccinMocha,
    Nord,
    Gruvbox,
    SolarizedDark,
    SolarizedLight,
    Custom(String),
}

impl ThemeKind {
    pub fn name(&self) -> &str {
        match self {
            Self::Dark => "dark",
            Self::CatppuccinMocha => "mocha",
            Self::Nord => "nord",
            Self::Gruvbox => "gruvbox",
            Self::SolarizedDark => "solarized-dark",
            Self::SolarizedLight => "solarized-light",
            Self::Custom(name) => name.as_str(),
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "dark" | "default" => Some(Self::Dark),
            "mocha" | "catppuccin" | "catppuccin-mocha" => Some(Self::CatppuccinMocha),
            "nord" => Some(Self::Nord),
            "gruvbox" => Some(Self::Gruvbox),
            "solarized-dark" | "solarized" => Some(Self::SolarizedDark),
            "solarized-light" => Some(Self::SolarizedLight),
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
            ThemeKind::Gruvbox => Self::gruvbox(),
            ThemeKind::SolarizedDark => Self::solarized_dark(),
            ThemeKind::SolarizedLight => Self::solarized_light(),
            ThemeKind::Custom(_) => Self::dark(), // fallback; use load_from_toml for custom
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

    // ── Gruvbox ─────────────────────────────────────────────────────────────
    pub fn gruvbox() -> Self {
        let dark0 = Color::Rgb(40, 40, 40); // #282828
        let dark1 = Color::Rgb(60, 56, 54); // #3c3836
        let dark2 = Color::Rgb(80, 73, 69); // #504945
        let dark3 = Color::Rgb(102, 92, 86); // #665c54
        let light0 = Color::Rgb(213, 196, 161); // #d5c4a1
        let light1 = Color::Rgb(235, 219, 178); // #ebdbb2
        let red = Color::Rgb(204, 102, 102); // #cc241d
        let green = Color::Rgb(152, 151, 26); // #98971a
        let yellow = Color::Rgb(181, 137, 0); // #b5a505
        let blue = Color::Rgb(104, 151, 187); // #6891b3
        let purple = Color::Rgb(177, 98, 134); // #b16286
        let aqua = Color::Rgb(104, 157, 106); // #689d6a
        let orange = Color::Rgb(211, 123, 69); // #d38609

        Self {
            bg: dark0,
            bg_surface: dark1,
            bg_highlight: dark2,
            bg_header: dark1,
            bg_footer: dark1,

            primary: blue,
            secondary: purple,

            border: dark3,
            border_focused: aqua,

            text: light1,
            text_secondary: light0,
            text_dimmed: dark3,

            cursor_normal_bg: yellow,
            cursor_insert_bg: purple,
            cursor_visual_bg: blue,
            cursor_fg: dark0,

            status_success: green,
            status_warning: yellow,
            status_error: red,
            status_info: blue,

            note_color: blue,
            inst_color: yellow,
            vol_color: purple,
            eff_color: orange,
        }
    }

    // ── Solarized Dark ─────────────────────────────────────────────────────
    pub fn solarized_dark() -> Self {
        let base03 = Color::Rgb(0, 43, 54); // #002b36
        let base02 = Color::Rgb(7, 54, 66); // #073642
        let base01 = Color::Rgb(88, 110, 117); // #586e75
        let _base00 = Color::Rgb(101, 123, 131); // #657b83
        let base0 = Color::Rgb(131, 148, 150); // #839496
        let base1 = Color::Rgb(147, 161, 161); // #93a1a1
        let yellow = Color::Rgb(181, 137, 0); // #b58900
        let orange = Color::Rgb(203, 75, 22); // #cb4b16
        let red = Color::Rgb(220, 50, 47); // #dc322f
        let magenta = Color::Rgb(211, 54, 130); // #d33682
        let violet = Color::Rgb(108, 113, 196); // #6c71c4
        let blue = Color::Rgb(38, 139, 210); // #268bd2
        let cyan = Color::Rgb(42, 161, 152); // #2aa198
        let green = Color::Rgb(133, 153, 0); // #859900

        Self {
            bg: base03,
            bg_surface: base02,
            bg_highlight: base02,
            bg_header: base02,
            bg_footer: base02,

            primary: blue,
            secondary: cyan,

            border: base01,
            border_focused: blue,

            text: base1,
            text_secondary: base0,
            text_dimmed: base01,

            cursor_normal_bg: yellow,
            cursor_insert_bg: magenta,
            cursor_visual_bg: violet,
            cursor_fg: base03,

            status_success: green,
            status_warning: yellow,
            status_error: red,
            status_info: cyan,

            note_color: blue,
            inst_color: yellow,
            vol_color: magenta,
            eff_color: orange,
        }
    }

    // ── Solarized Light ───────────────────────────────────────────────────
    pub fn solarized_light() -> Self {
        let base2 = Color::Rgb(238, 232, 213); // #eee8d5
        let base3 = Color::Rgb(253, 246, 227); // #fdf6e3
        let base1 = Color::Rgb(147, 161, 161); // #93a1a1
        let base0 = Color::Rgb(131, 148, 150); // #839496
        let base00 = Color::Rgb(101, 123, 131); // #657b83
        let _base01 = Color::Rgb(88, 110, 117); // #586e75
        let yellow = Color::Rgb(181, 137, 0); // #b58900
        let orange = Color::Rgb(203, 75, 22); // #cb4b16
        let red = Color::Rgb(220, 50, 47); // #dc322f
        let magenta = Color::Rgb(211, 54, 130); // #d33682
        let violet = Color::Rgb(108, 113, 196); // #6c71c4
        let blue = Color::Rgb(38, 139, 210); // #268bd2
        let cyan = Color::Rgb(42, 161, 152); // #2aa198
        let green = Color::Rgb(133, 153, 0); // #859900

        Self {
            bg: base3,
            bg_surface: base2,
            bg_highlight: base2,
            bg_header: base2,
            bg_footer: base2,

            primary: blue,
            secondary: cyan,

            border: base1,
            border_focused: blue,

            text: base00,
            text_secondary: base0,
            text_dimmed: base1,

            cursor_normal_bg: yellow,
            cursor_insert_bg: magenta,
            cursor_visual_bg: violet,
            cursor_fg: base3,

            status_success: green,
            status_warning: yellow,
            status_error: red,
            status_info: cyan,

            note_color: blue,
            inst_color: yellow,
            vol_color: magenta,
            eff_color: orange,
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

    /// Load a theme from a TOML file.
    ///
    /// The TOML file should contain `[colors]` table with keys matching the
    /// Theme struct field names, each with an RGB hex string like `"#RRGGBB"`.
    /// Missing fields fall back to the dark theme defaults.
    pub fn load_from_toml(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read theme file: {}", e))?;
        let table: toml::Table = content
            .parse()
            .map_err(|e| format!("Failed to parse theme TOML: {}", e))?;

        let colors = table
            .get("colors")
            .and_then(|v| v.as_table())
            .ok_or_else(|| "Missing [colors] table in theme file".to_string())?;

        let mut theme = Self::dark();

        fn parse_color(val: &toml::Value) -> Option<Color> {
            let s = val.as_str()?;
            if s.len() == 7 && s.starts_with('#') {
                let r = u8::from_str_radix(&s[1..3], 16).ok()?;
                let g = u8::from_str_radix(&s[3..5], 16).ok()?;
                let b = u8::from_str_radix(&s[5..7], 16).ok()?;
                Some(Color::Rgb(r, g, b))
            } else {
                None
            }
        }

        macro_rules! set_color {
            ($field:ident) => {
                if let Some(val) = colors.get(stringify!($field)) {
                    if let Some(color) = parse_color(val) {
                        theme.$field = color;
                    }
                }
            };
        }

        set_color!(bg);
        set_color!(bg_surface);
        set_color!(bg_highlight);
        set_color!(bg_header);
        set_color!(bg_footer);
        set_color!(primary);
        set_color!(secondary);
        set_color!(border);
        set_color!(border_focused);
        set_color!(text);
        set_color!(text_secondary);
        set_color!(text_dimmed);
        set_color!(cursor_normal_bg);
        set_color!(cursor_insert_bg);
        set_color!(cursor_visual_bg);
        set_color!(cursor_fg);
        set_color!(status_success);
        set_color!(status_warning);
        set_color!(status_error);
        set_color!(status_info);
        set_color!(note_color);
        set_color!(inst_color);
        set_color!(vol_color);
        set_color!(eff_color);

        Ok(theme)
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
        for kind in [
            ThemeKind::Dark,
            ThemeKind::CatppuccinMocha,
            ThemeKind::Nord,
            ThemeKind::Gruvbox,
            ThemeKind::SolarizedDark,
            ThemeKind::SolarizedLight,
        ] {
            let t = Theme::from_kind(kind.clone());
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
        assert_eq!(ThemeKind::from_str("gruvbox"), Some(ThemeKind::Gruvbox));
        assert_eq!(
            ThemeKind::from_str("solarized-dark"),
            Some(ThemeKind::SolarizedDark)
        );
        assert_eq!(
            ThemeKind::from_str("solarized"),
            Some(ThemeKind::SolarizedDark)
        );
        assert_eq!(
            ThemeKind::from_str("solarized-light"),
            Some(ThemeKind::SolarizedLight)
        );
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
        for t in [
            Theme::dark(),
            Theme::catppuccin_mocha(),
            Theme::nord(),
            Theme::gruvbox(),
            Theme::solarized_dark(),
            Theme::solarized_light(),
        ] {
            let _ = t.note_color;
            let _ = t.inst_color;
            let _ = t.vol_color;
            let _ = t.eff_color;
        }
    }

    #[test]
    fn test_gruvbox_theme_has_rgb_colors() {
        let t = Theme::gruvbox();
        assert!(matches!(t.bg, Color::Rgb(_, _, _)));
        assert!(matches!(t.primary, Color::Rgb(_, _, _)));
    }

    #[test]
    fn test_solarized_dark_theme_has_rgb_colors() {
        let t = Theme::solarized_dark();
        assert!(matches!(t.bg, Color::Rgb(_, _, _)));
        assert!(matches!(t.primary, Color::Rgb(_, _, _)));
    }

    #[test]
    fn test_solarized_light_theme_has_rgb_colors() {
        let t = Theme::solarized_light();
        assert!(matches!(t.bg, Color::Rgb(_, _, _)));
        assert!(matches!(t.primary, Color::Rgb(_, _, _)));
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

    #[test]
    fn test_load_from_toml_valid() {
        let dir = std::env::temp_dir().join("riffl_test_theme");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_theme.toml");
        std::fs::write(
            &path,
            r##"
[colors]
bg = "#1a1a2e"
primary = "#e94560"
text = "#ffffff"
"##,
        )
        .unwrap();

        let theme = Theme::load_from_toml(&path).unwrap();
        assert_eq!(theme.bg, Color::Rgb(0x1a, 0x1a, 0x2e));
        assert_eq!(theme.primary, Color::Rgb(0xe9, 0x45, 0x60));
        assert_eq!(theme.text, Color::Rgb(0xff, 0xff, 0xff));
        // Non-specified fields should be dark theme defaults
        assert_eq!(theme.secondary, Theme::dark().secondary);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_from_toml_missing_colors_table() {
        let dir = std::env::temp_dir().join("riffl_test_theme2");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad_theme.toml");
        std::fs::write(&path, "title = \"no colors\"").unwrap();

        let result = Theme::load_from_toml(&path);
        assert!(result.is_err());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_from_toml_nonexistent_file() {
        let result = Theme::load_from_toml(Path::new("/nonexistent/theme.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_theme_kind_custom() {
        let kind = ThemeKind::Custom("cyberpunk".to_string());
        assert_eq!(kind.name(), "cyberpunk");
        let theme = Theme::from_kind(kind);
        // Custom falls back to dark
        assert_eq!(theme, Theme::dark());
    }
}
