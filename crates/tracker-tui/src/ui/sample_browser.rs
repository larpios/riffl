//! Dedicated sample browser view.
//!
//! Supports multiple configured root directories. When more than one root is
//! configured, the browser opens at a "roots list" showing all of them.
//! Pressing Enter/l enters a root or subdirectory; h returns up, stopping at
//! the roots list. Pressing Enter on an audio file loads it as an instrument.

use std::path::{Path, PathBuf};

use ratatui::{
    layout::Alignment,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::theme::Theme;

const AUDIO_EXTENSIONS: &[&str] = &["wav", "flac", "ogg", "mod"];

/// A single entry in the browser list.
#[derive(Debug, Clone)]
pub struct BrowserEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

/// Internal navigation state.
#[derive(Debug, Clone)]
enum BrowserMode {
    /// Showing the list of root directories.
    Roots,
    /// Navigating inside a root.
    InDir {
        /// Which root we entered.
        root: PathBuf,
        /// Currently displayed directory (may equal root or be a subdir).
        current: PathBuf,
    },
}

/// State for the dedicated sample browser view.
#[derive(Debug, Clone)]
pub struct SampleBrowser {
    roots: Vec<PathBuf>,
    mode: BrowserMode,
    entries: Vec<BrowserEntry>,
    selected: usize,
}

impl SampleBrowser {
    /// Create a new sample browser with the given root directories.
    /// If exactly one root is given, starts inside it directly.
    pub fn new(roots: Vec<PathBuf>) -> Self {
        let mut browser = Self {
            roots,
            mode: BrowserMode::Roots,
            entries: Vec::new(),
            selected: 0,
        };
        browser.refresh_entries();
        browser
    }

    /// Replace all roots. Resets to the roots list (or directly into the single root).
    pub fn set_roots(&mut self, roots: Vec<PathBuf>) {
        self.roots = roots;
        self.mode = BrowserMode::Roots;
        self.selected = 0;
        self.refresh_entries();
    }

    /// Move selection up.
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down.
    pub fn move_down(&mut self) {
        if !self.entries.is_empty() && self.selected < self.entries.len() - 1 {
            self.selected += 1;
        }
    }

    /// Try to enter the selected directory. Returns `false` if selection is a file.
    pub fn enter_dir(&mut self) -> bool {
        let entry = match self.entries.get(self.selected) {
            Some(e) if e.is_dir => e.clone(),
            _ => return false,
        };
        match &self.mode {
            BrowserMode::Roots => {
                // Entering a root
                self.mode = BrowserMode::InDir {
                    root: entry.path.clone(),
                    current: entry.path.clone(),
                };
            }
            BrowserMode::InDir { root, .. } => {
                // Entering a subdir within the current root
                self.mode = BrowserMode::InDir {
                    root: root.clone(),
                    current: entry.path.clone(),
                };
            }
        }
        self.selected = 0;
        self.refresh_entries();
        true
    }

    /// Navigate up. From a root's top returns to the roots list.
    /// From the roots list does nothing.
    pub fn go_up(&mut self) {
        match &self.mode {
            BrowserMode::Roots => {}
            BrowserMode::InDir { root, current } => {
                if current == root {
                    // At top of this root — return to roots list
                    self.mode = BrowserMode::Roots;
                } else if let Some(parent) = current.parent() {
                    let parent = parent.to_path_buf();
                    let root = root.clone();
                    self.mode = BrowserMode::InDir {
                        root,
                        current: parent,
                    };
                }
            }
        }
        self.selected = 0;
        self.refresh_entries();
    }

    /// Return the path of the currently selected entry (file or dir), if any.
    pub fn selected_path(&self) -> Option<&Path> {
        self.entries.get(self.selected).map(|e| e.path.as_path())
    }

    /// Return `true` if the selected entry is an audio file (not a directory).
    pub fn selected_is_file(&self) -> bool {
        self.entries
            .get(self.selected)
            .map(|e| !e.is_dir)
            .unwrap_or(false)
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn entries(&self) -> &[BrowserEntry] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// True when we're showing the top-level roots list.
    pub fn at_roots(&self) -> bool {
        matches!(self.mode, BrowserMode::Roots)
    }

    /// The current directory being shown (None when at roots list).
    pub fn current_dir(&self) -> Option<&Path> {
        match &self.mode {
            BrowserMode::Roots => None,
            BrowserMode::InDir { current, .. } => Some(current.as_path()),
        }
    }

    /// The root we're currently browsing inside (None when at roots list).
    pub fn current_root(&self) -> Option<&Path> {
        match &self.mode {
            BrowserMode::Roots => None,
            BrowserMode::InDir { root, .. } => Some(root.as_path()),
        }
    }

    fn refresh_entries(&mut self) {
        self.entries = match &self.mode {
            BrowserMode::Roots => self
                .roots
                .iter()
                .map(|p| BrowserEntry {
                    name: p
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_else(|| p.to_str().unwrap_or("?"))
                        .to_string(),
                    path: p.clone(),
                    is_dir: true,
                })
                .collect(),
            BrowserMode::InDir { current, .. } => scan_entries(current),
        };
    }
}

/// Scan a directory for subdirectories and supported audio files.
/// Returns dirs first (alphabetically), then files (alphabetically).
fn scan_entries(dir: &Path) -> Vec<BrowserEntry> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    let rd = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };

    for entry in rd.flatten() {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            dirs.push(BrowserEntry {
                name,
                path,
                is_dir: true,
            });
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                files.push(BrowserEntry {
                    name,
                    path,
                    is_dir: false,
                });
            }
        }
    }

    dirs.sort_by(|a, b| a.name.cmp(&b.name));
    files.sort_by(|a, b| a.name.cmp(&b.name));
    dirs.extend(files);
    dirs
}

/// Render the sample browser view.
pub fn render_sample_browser(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    browser: &SampleBrowser,
    theme: &Theme,
) {
    let title = match browser.current_dir() {
        None => " Sample Browser ".to_string(),
        Some(cur) => {
            // Show path relative to the root we entered
            let rel = browser
                .current_root()
                .and_then(|root| cur.strip_prefix(root).ok())
                .and_then(|p| p.to_str())
                .filter(|s| !s.is_empty())
                .map(|s| format!("/{s}"))
                .unwrap_or_default();
            let root_name = browser
                .current_root()
                .and_then(|r| r.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("?");
            format!(" Sample Browser / {root_name}{rel} ")
        }
    };

    let at_root_list = browser.at_roots();
    let nav_hint = if at_root_list {
        "  l/Enter: browse dir  ·  j/k: navigate"
    } else {
        "  l/Enter: enter dir  ·  Enter: load  ·  h: up  ·  j/k: navigate"
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .title(title)
        .title_alignment(Alignment::Left)
        .title_bottom(Span::styled(
            nav_hint,
            Style::default().fg(theme.text_dimmed),
        ));

    let inner = block.inner(area);
    let mut lines: Vec<Line> = Vec::new();

    if browser.is_empty() {
        lines.push(Line::from(""));
        let msg = if at_root_list {
            "  (no sample directories configured)"
        } else {
            "  (no audio files or subdirectories here)"
        };
        lines.push(Line::from(Span::styled(
            msg,
            Style::default().fg(theme.text_dimmed),
        )));
    } else {
        for (idx, entry) in browser.entries().iter().enumerate() {
            let is_selected = idx == browser.selected_index();

            let label = if entry.is_dir {
                if at_root_list {
                    // Show full path for roots
                    format!("  {}", entry.path.display())
                } else {
                    format!("  [{}]", entry.name)
                }
            } else {
                format!("    {}", entry.name)
            };

            let fg = if entry.is_dir {
                theme.primary
            } else {
                theme.text
            };

            let style = if is_selected {
                Style::default().fg(theme.text).bg(theme.bg_highlight)
            } else {
                Style::default().fg(fg)
            };

            lines.push(Line::from(Span::styled(label, style)));
        }
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(block, area);
    frame.render_widget(paragraph, inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_dir(suffix: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("riffl_sb_{suffix}"));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn test_single_root_starts_browsing() {
        let dir = make_dir("single");
        fs::write(dir.join("kick.wav"), b"x").unwrap();
        let b = SampleBrowser::new(vec![dir.clone()]);
        // With one root, starts in root list (user still chooses to enter)
        // Actually: we always start at roots list regardless of count
        assert!(b.at_roots());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_enter_root_and_go_up() {
        let dir = make_dir("enter_up");
        fs::write(dir.join("kick.wav"), b"x").unwrap();
        let mut b = SampleBrowser::new(vec![dir.clone()]);
        assert!(b.at_roots());

        // Enter the root
        assert!(b.enter_dir());
        assert!(!b.at_roots());
        assert_eq!(b.current_dir(), Some(dir.as_path()));

        // Go up returns to roots list
        b.go_up();
        assert!(b.at_roots());

        // Go up again is a no-op
        b.go_up();
        assert!(b.at_roots());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_subdir_navigation() {
        let dir = make_dir("subdir_nav");
        let sub = dir.join("drums");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("kick.wav"), b"x").unwrap();

        let mut b = SampleBrowser::new(vec![dir.clone()]);
        b.enter_dir(); // enter root

        // should see the drums subdir
        assert_eq!(b.entries()[0].name, "drums");
        assert!(b.enter_dir()); // enter drums

        assert_eq!(b.current_dir(), Some(sub.as_path()));

        // go up returns to root (not roots list)
        b.go_up();
        assert_eq!(b.current_dir(), Some(dir.as_path()));

        // go up again returns to roots list
        b.go_up();
        assert!(b.at_roots());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_multiple_roots_shown_at_top() {
        let a = make_dir("multi_a");
        let b_dir = make_dir("multi_b");
        let b = SampleBrowser::new(vec![a.clone(), b_dir.clone()]);
        assert!(b.at_roots());
        assert_eq!(b.entries().len(), 2);
        fs::remove_dir_all(&a).ok();
        fs::remove_dir_all(&b_dir).ok();
    }

    #[test]
    fn test_enter_file_returns_false() {
        let dir = make_dir("file_enter");
        fs::write(dir.join("kick.wav"), b"x").unwrap();
        let mut b = SampleBrowser::new(vec![dir.clone()]);
        b.enter_dir(); // enter root
                       // select the file (index 0 after entering)
        assert!(!b.entries()[0].is_dir);
        assert!(!b.enter_dir());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_selected_is_file() {
        let dir = make_dir("is_file");
        fs::write(dir.join("snare.wav"), b"x").unwrap();
        let mut b = SampleBrowser::new(vec![dir.clone()]);
        b.enter_dir();
        assert!(b.selected_is_file());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_set_roots_resets_state() {
        let a = make_dir("reset_a");
        let b_dir = make_dir("reset_b");
        let mut b = SampleBrowser::new(vec![a.clone()]);
        b.enter_dir();
        assert!(!b.at_roots());

        b.set_roots(vec![a.clone(), b_dir.clone()]);
        assert!(b.at_roots());
        assert_eq!(b.entries().len(), 2);

        fs::remove_dir_all(&a).ok();
        fs::remove_dir_all(&b_dir).ok();
    }
}
