//! Dedicated sample browser view.
//!
//! Supports multiple configured root directories. When more than one root is
//! configured, the browser opens at a "roots list" showing all of them.
//! Pressing Enter/l enters a root or subdirectory; h navigates up — going
//! above a configured root escapes into the real filesystem all the way to /.
//! Press ~ to jump back to the roots list from anywhere.
//! Pressing Enter on an audio file loads it as an instrument.

use std::path::{Path, PathBuf};

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
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
    /// True when this root entry comes from the configured (pinned) roots list.
    pub is_pinned: bool,
    /// True when this root entry is in the user's bookmarks.
    pub is_bookmarked: bool,
}

/// Internal navigation state.
#[derive(Debug, Clone)]
enum BrowserMode {
    /// Showing the list of root directories.
    Roots,
    /// Navigating inside a directory (may be above the configured root).
    InDir {
        /// The configured root we originally entered (kept for display/context).
        root: PathBuf,
        /// Currently displayed directory (may be above `root`).
        current: PathBuf,
    },
}

/// State for the dedicated sample browser view.
#[derive(Debug, Clone)]
pub struct SampleBrowser {
    /// Configured (pinned) roots — set via [`new`] / [`set_roots`].
    pinned_roots: Vec<PathBuf>,
    /// All roots, including auto-detected ones added via [`add_auto_root`].
    roots: Vec<PathBuf>,
    /// User-bookmarked directories, shown at the top of the roots list.
    bookmarked_paths: Vec<PathBuf>,
    mode: BrowserMode,
    entries: Vec<BrowserEntry>,
    selected: usize,
    /// Cached waveform peak amplitudes for the currently selected audio file.
    /// Empty when no waveform is loaded or selection is a directory.
    pub waveform_peaks: Vec<f32>,
    /// The file path whose waveform is currently cached.
    pub waveform_path: Option<PathBuf>,
}

impl SampleBrowser {
    /// Create a new sample browser with the given root directories.
    /// All supplied roots are treated as "pinned" (configured).
    pub fn new(roots: Vec<PathBuf>) -> Self {
        let mut browser = Self {
            pinned_roots: roots.clone(),
            roots,
            bookmarked_paths: Vec::new(),
            mode: BrowserMode::Roots,
            entries: Vec::new(),
            selected: 0,
            waveform_peaks: Vec::new(),
            waveform_path: None,
        };
        browser.refresh_entries();
        browser
    }

    /// Replace all configured (pinned) roots. Resets to the roots list.
    /// Any previously added auto-roots are discarded.
    pub fn set_roots(&mut self, roots: Vec<PathBuf>) {
        self.pinned_roots = roots.clone();
        self.roots = roots;
        self.mode = BrowserMode::Roots;
        self.selected = 0;
        self.refresh_entries();
    }

    /// Add a non-pinned (auto-detected) root directory.
    /// Has no effect if the path is already present.
    pub fn add_auto_root(&mut self, path: PathBuf) {
        if !self.roots.contains(&path) {
            self.roots.push(path);
            if matches!(self.mode, BrowserMode::Roots) {
                self.refresh_entries();
            }
        }
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
                // Entering a subdir — keep root for context
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

    /// Navigate up one directory level.
    ///
    /// Unlike the old behaviour this **does not stop at the configured root
    /// boundary** — pressing `h` past a configured root continues up the
    /// real filesystem, stopping only at the filesystem root (no parent).
    /// To return to the virtual roots list call [`reset_to_roots`].
    pub fn go_up(&mut self) {
        match &self.mode {
            BrowserMode::Roots => {} // already at top-level roots list
            BrowserMode::InDir { root, current } => {
                // Navigate to parent regardless of whether we're at the
                // configured root boundary — stop only at filesystem root.
                if let Some(parent) = current.parent() {
                    let parent = parent.to_path_buf();
                    let root = root.clone();
                    self.mode = BrowserMode::InDir {
                        root,
                        current: parent,
                    };
                }
                // If current.parent() is None we're at / — stay put.
            }
        }
        self.selected = 0;
        self.refresh_entries();
    }

    /// Jump back to the roots list from anywhere in the filesystem.
    pub fn reset_to_roots(&mut self) {
        self.mode = BrowserMode::Roots;
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

    /// Return `true` if the selected entry is a directory.
    pub fn selected_is_dir(&self) -> bool {
        self.entries
            .get(self.selected)
            .map(|e| e.is_dir)
            .unwrap_or(false)
    }

    /// Return `true` if the selected directory is currently bookmarked.
    pub fn selected_is_bookmarked(&self) -> bool {
        match self.entries.get(self.selected) {
            Some(e) if e.is_dir => self.bookmarked_paths.contains(&e.path),
            _ => false,
        }
    }

    /// Store computed waveform peaks for the given file path.
    pub fn set_waveform_peaks(&mut self, path: PathBuf, peaks: Vec<f32>) {
        self.waveform_path = Some(path);
        self.waveform_peaks = peaks;
    }

    /// Clear cached waveform data (called when selection moves to a non-WAV entry).
    pub fn clear_waveform(&mut self) {
        self.waveform_path = None;
        self.waveform_peaks.clear();
    }

    /// The file path whose waveform peaks are currently cached, if any.
    pub fn waveform_path(&self) -> Option<&Path> {
        self.waveform_path.as_deref()
    }

    /// Cached waveform peak amplitudes (empty when none loaded).
    pub fn waveform_peaks(&self) -> &[f32] {
        &self.waveform_peaks
    }

    /// Set the bookmarked directories. Bookmarked dirs appear at the top of the
    /// roots list. Refreshes entries if currently showing the roots list.
    pub fn set_bookmarks(&mut self, paths: Vec<PathBuf>) {
        self.bookmarked_paths = paths;
        if matches!(self.mode, BrowserMode::Roots) {
            self.refresh_entries();
        }
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// Set the selected index (clamped to valid range). Primarily for tests.
    pub fn select(&mut self, idx: usize) {
        self.selected = idx.min(self.entries.len().saturating_sub(1));
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

    /// The configured root we originally entered (None when at roots list).
    pub fn current_root(&self) -> Option<&Path> {
        match &self.mode {
            BrowserMode::Roots => None,
            BrowserMode::InDir { root, .. } => Some(root.as_path()),
        }
    }

    fn refresh_entries(&mut self) {
        let pinned = &self.pinned_roots;
        let bookmarked = &self.bookmarked_paths;
        self.entries = match &self.mode {
            BrowserMode::Roots => {
                // Bookmarked dirs first (alphabetically), then non-bookmarked roots in original order.
                let mut bm_entries: Vec<BrowserEntry> = bookmarked
                    .iter()
                    .map(|p| BrowserEntry {
                        name: p
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or_else(|| p.to_str().unwrap_or("?"))
                            .to_string(),
                        path: p.clone(),
                        is_dir: true,
                        is_pinned: pinned.contains(p),
                        is_bookmarked: true,
                    })
                    .collect();
                bm_entries.sort_by(|a, b| a.name.cmp(&b.name));

                let rest: Vec<BrowserEntry> = self
                    .roots
                    .iter()
                    .filter(|p| !bookmarked.contains(p))
                    .map(|p| BrowserEntry {
                        name: p
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or_else(|| p.to_str().unwrap_or("?"))
                            .to_string(),
                        path: p.clone(),
                        is_dir: true,
                        is_pinned: pinned.contains(p),
                        is_bookmarked: false,
                    })
                    .collect();

                bm_entries.extend(rest);
                bm_entries
            }
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
                is_pinned: false,
                is_bookmarked: false,
            });
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                files.push(BrowserEntry {
                    name,
                    path,
                    is_dir: false,
                    is_pinned: false,
                    is_bookmarked: false,
                });
            }
        }
    }

    dirs.sort_by(|a, b| a.name.cmp(&b.name));
    files.sort_by(|a, b| a.name.cmp(&b.name));
    dirs.extend(files);
    dirs
}

/// Compute downsampled peak amplitudes for a WAV file.
///
/// Returns `n_bars` peak values (each 0.0..=1.0) suitable for rendering a
/// waveform thumbnail. Returns an empty `Vec` when the file cannot be decoded.
pub fn compute_waveform_peaks(path: &Path, n_bars: usize) -> Vec<f32> {
    if n_bars == 0 {
        return Vec::new();
    }
    // Use a fixed target rate — we only need amplitudes, not correct pitch.
    let sample = match tracker_core::audio::load_sample(path, 44100) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let frames = sample.frame_count();
    if frames == 0 {
        return Vec::new();
    }
    let channels = sample.channels() as usize;
    let data = sample.data();
    let frames_per_bar = frames.div_ceil(n_bars);
    let mut peaks = Vec::with_capacity(n_bars);
    for bar in 0..n_bars {
        let start = bar * frames_per_bar;
        let end = (start + frames_per_bar).min(frames);
        if start >= end {
            peaks.push(0.0f32);
            continue;
        }
        let mut peak = 0.0f32;
        for frame in start..end {
            for ch in 0..channels {
                let idx = frame * channels + ch;
                if idx < data.len() {
                    peak = peak.max(data[idx].abs());
                }
            }
        }
        peaks.push(peak);
    }
    peaks
}

/// Build the window title for the sample browser.
///
/// Shows `" Sample Browser / {root_name} "` when at the configured root,
/// `" Sample Browser / {root_name}/{rel} "` when inside it, and
/// `" Sample Browser / {dir_name} "` when navigated above all roots
/// (using only `file_name()` to avoid an absolute-path double-slash).
fn browser_title(browser: &SampleBrowser) -> String {
    match browser.current_dir() {
        None => " Sample Browser ".to_string(),
        Some(cur) => {
            let path_str = browser
                .current_root()
                .and_then(|root| {
                    let root_name = root.file_name()?.to_str()?;
                    let rel = cur.strip_prefix(root).ok()?.to_str()?;
                    if rel.is_empty() {
                        // Exactly at the configured root — show just the root name.
                        Some(root_name.to_string())
                    } else {
                        Some(format!("{root_name}/{rel}"))
                    }
                })
                .unwrap_or_else(|| {
                    // Navigated above all configured roots.  Use file_name() to
                    // avoid an absolute path producing a double-slash in the title.
                    cur.file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| cur.display().to_string())
                });
            format!(" Sample Browser / {path_str} ")
        }
    }
}

/// Format a duration (in seconds) as `MM:SS.cc` (centiseconds).
pub(crate) fn format_preview_time(secs: f64) -> String {
    let total_cs = (secs * 100.0) as u64;
    let cs = total_cs % 100;
    let total_s = total_cs / 100;
    let s = total_s % 60;
    let m = total_s / 60;
    format!("{m:02}:{s:02}.{cs:02}")
}

/// Compute the cursor column for a given playback position within a bar-chart panel.
///
/// Returns the column index (0-based, clamped to `[0, width.saturating_sub(1)]`)
/// that corresponds to `pos` out of `total` frames mapped onto `width` columns.
/// Returns `0` when `total == 0`.
pub(crate) fn cursor_col_for(pos: usize, total: usize, width: usize) -> usize {
    if total == 0 || width == 0 {
        return 0;
    }
    ((pos * width) / total).min(width.saturating_sub(1))
}

/// Render the sample browser view.
///
/// When a WAV waveform has been lazy-loaded (`browser.waveform_peaks` is
/// non-empty) and the terminal is wide enough, the area is split horizontally:
/// ~65 % for the file list on the left and ~35 % for the waveform panel on
/// the right.
///
/// `preview_pos` and `total_frames` are the current playback position and
/// total frame count (both in output-rate frames) for the active browser preview.
/// `output_sample_rate` converts frame counts to wall-clock time.
pub fn render_sample_browser(
    frame: &mut Frame,
    area: Rect,
    browser: &SampleBrowser,
    theme: &Theme,
    preview_pos: usize,
    total_frames: usize,
    output_sample_rate: u32,
) {
    let show_waveform =
        !browser.waveform_peaks.is_empty() && browser.waveform_path.is_some() && area.width >= 60;

    let (list_area, wave_area_opt) = if show_waveform {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    render_browser_list(frame, list_area, browser, theme);

    if let (Some(wave_area), Some(wpath)) = (wave_area_opt, browser.waveform_path.as_deref()) {
        render_waveform_panel(
            frame,
            wave_area,
            &browser.waveform_peaks,
            wpath,
            theme,
            preview_pos,
            total_frames,
            output_sample_rate,
        );
    }
}

/// Render the file-list portion of the sample browser.
fn render_browser_list(frame: &mut Frame, area: Rect, browser: &SampleBrowser, theme: &Theme) {
    let title = browser_title(browser);

    let at_root_list = browser.at_roots();
    let nav_hint = if at_root_list {
        "  l/Enter: browse dir  ·  b: bookmark  ·  j/k: navigate"
    } else {
        "  Space: preview/stop  ·  ←/→: scrub  ·  Enter: load  ·  l: enter dir  ·  b: bookmark  ·  h: up  ·  ~: roots  ·  j/k: navigate"
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
                    // Bookmarked roots get ♥, pinned (configured) roots get ★.
                    let pin = if entry.is_bookmarked {
                        "\u{2665} "
                    } else if entry.is_pinned {
                        "\u{2605} "
                    } else {
                        "  "
                    };
                    format!("  {pin}{}", entry.path.display())
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

/// Render a waveform bar-chart panel for a selected audio file.
///
/// Uses eighth-block characters (`▁▂▃▄▅▆▇█`) to draw a bar chart from the
/// bottom of the panel, one column per time slice, giving sub-row amplitude
/// resolution.
#[allow(clippy::too_many_arguments)]
fn render_waveform_panel(
    frame: &mut Frame,
    area: Rect,
    peaks: &[f32],
    path: &Path,
    theme: &Theme,
    preview_pos: usize,
    total_frames: usize,
    output_sample_rate: u32,
) {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("waveform");
    let title = format!(" {name} ");

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .title(Span::styled(title, Style::default().fg(theme.text)))
        .title_alignment(Alignment::Center);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let h = inner.height as usize;
    let w = inner.width as usize;
    if h == 0 || w == 0 || peaks.is_empty() {
        return;
    }

    // Reserve the bottom row for the time display when a preview is active.
    let show_time = total_frames > 0 && output_sample_rate > 0;
    let waveform_rows = if show_time && h > 1 { h - 1 } else { h };

    // Compute cursor column (0-based within waveform columns).
    let cursor_col = if total_frames > 0 {
        Some(cursor_col_for(preview_pos, total_frames, w))
    } else {
        None
    };

    // Map the peak array to exactly `w` bars by index interpolation.
    let bars: Vec<f32> = (0..w)
        .map(|col| {
            let idx = (col * peaks.len()) / w;
            peaks[idx.min(peaks.len() - 1)]
        })
        .collect();

    // Eighth-block chars for sub-row precision (index 0 = empty, 8 = full).
    const EIGHTHS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    let mut lines: Vec<Line> = Vec::with_capacity(h);
    for row in 0..waveform_rows {
        // row 0 = top of panel; row_from_bottom 0 = bottom row.
        let row_from_bottom = waveform_rows - 1 - row;
        let subcells_below = row_from_bottom * 8;

        let spans: Vec<Span> = bars
            .iter()
            .enumerate()
            .map(|(col, &amp)| {
                let is_cursor = cursor_col == Some(col);
                let total = (amp * (waveform_rows * 8) as f32) as usize;
                let ch = if subcells_below + 8 <= total {
                    '█'
                } else if subcells_below < total {
                    EIGHTHS[(total - subcells_below).min(8)]
                } else {
                    ' '
                };
                let style = if is_cursor {
                    // Cursor column: bright accent, always visible.
                    Style::default().fg(theme.warning_color())
                } else if ch != ' ' {
                    Style::default().fg(theme.primary)
                } else {
                    Style::default()
                };
                let display = if is_cursor { '▏' } else { ch };
                Span::styled(display.to_string(), style)
            })
            .collect();
        lines.push(Line::from(spans));
    }

    // Time display row: "MM:SS.cc / MM:SS.cc" centred at the bottom.
    if show_time && h > waveform_rows {
        let rate = output_sample_rate as f64;
        let elapsed = format_preview_time(preview_pos as f64 / rate);
        let total_t = format_preview_time(total_frames as f64 / rate);
        let time_str = format!("{elapsed} / {total_t}");
        let time_span = Span::styled(time_str, Style::default().fg(theme.text_dimmed));
        lines.push(Line::from(vec![time_span]).centered());
    }

    frame.render_widget(Paragraph::new(lines), inner);
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
        // Always starts at the roots list
        assert!(b.at_roots());
        fs::remove_dir_all(&dir).ok();
    }

    // --- Root escape behaviour ---

    #[test]
    fn test_go_up_escapes_past_configured_root() {
        let base = make_dir("escape_base");
        let root = base.join("project_dir");
        fs::create_dir_all(&root).unwrap();

        let mut b = SampleBrowser::new(vec![root.clone()]);
        assert!(b.at_roots());

        // Enter the configured root
        assert!(b.enter_dir());
        assert!(!b.at_roots());
        assert_eq!(b.current_dir(), Some(root.as_path()));

        // go_up should navigate to the parent of the root, NOT back to roots list
        b.go_up();
        assert!(
            !b.at_roots(),
            "should still be in InDir, not back at roots list"
        );
        assert_eq!(b.current_dir(), Some(base.as_path()));

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn test_go_up_two_levels_above_root() {
        let grandparent = make_dir("gp");
        let parent = grandparent.join("parent");
        let root = parent.join("root");
        fs::create_dir_all(&root).unwrap();

        let mut b = SampleBrowser::new(vec![root.clone()]);
        b.enter_dir();
        assert_eq!(b.current_dir(), Some(root.as_path()));

        b.go_up(); // → parent
        assert_eq!(b.current_dir(), Some(parent.as_path()));

        b.go_up(); // → grandparent
        assert_eq!(b.current_dir(), Some(grandparent.as_path()));

        assert!(!b.at_roots());

        fs::remove_dir_all(&grandparent).ok();
    }

    // --- reset_to_roots ---

    #[test]
    fn test_reset_to_roots_from_indir() {
        let dir = make_dir("reset_indir");
        let mut b = SampleBrowser::new(vec![dir.clone()]);
        b.enter_dir();
        assert!(!b.at_roots());

        b.reset_to_roots();
        assert!(b.at_roots());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_reset_to_roots_when_already_at_roots() {
        let dir = make_dir("reset_already");
        let mut b = SampleBrowser::new(vec![dir.clone()]);
        assert!(b.at_roots());
        b.reset_to_roots(); // should be a no-op
        assert!(b.at_roots());
        fs::remove_dir_all(&dir).ok();
    }

    // --- Pinned roots ---

    #[test]
    fn test_configured_root_is_pinned() {
        let dir = make_dir("pinned_flag");
        let b = SampleBrowser::new(vec![dir.clone()]);
        assert!(b.at_roots());
        assert_eq!(b.entries().len(), 1);
        assert!(b.entries()[0].is_pinned, "configured root should be pinned");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_auto_root_not_pinned() {
        let configured = make_dir("auto_configured");
        let auto = make_dir("auto_detected");

        let mut b = SampleBrowser::new(vec![configured.clone()]);
        b.add_auto_root(auto.clone());

        assert!(b.at_roots());
        assert_eq!(b.entries().len(), 2);

        let conf_entry = b.entries().iter().find(|e| e.path == configured).unwrap();
        let auto_entry = b.entries().iter().find(|e| e.path == auto).unwrap();

        assert!(conf_entry.is_pinned, "configured root should be pinned");
        assert!(
            !auto_entry.is_pinned,
            "auto-detected root should not be pinned"
        );

        fs::remove_dir_all(&configured).ok();
        fs::remove_dir_all(&auto).ok();
    }

    #[test]
    fn test_add_auto_root_dedup() {
        let dir = make_dir("auto_dedup");
        let mut b = SampleBrowser::new(vec![dir.clone()]);
        b.add_auto_root(dir.clone());
        assert_eq!(b.entries().len(), 1, "should not add duplicate root");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_set_roots_clears_auto_roots() {
        let a = make_dir("setroots_a");
        let auto = make_dir("setroots_auto");
        let b_dir = make_dir("setroots_b");

        let mut b = SampleBrowser::new(vec![a.clone()]);
        b.add_auto_root(auto.clone());
        assert_eq!(b.entries().len(), 2);

        // set_roots discards the auto root
        b.set_roots(vec![b_dir.clone()]);
        assert_eq!(b.entries().len(), 1);
        assert_eq!(b.entries()[0].path, b_dir);

        fs::remove_dir_all(&a).ok();
        fs::remove_dir_all(&auto).ok();
        fs::remove_dir_all(&b_dir).ok();
    }

    // --- Existing behaviour preserved ---

    #[test]
    fn test_enter_root_and_reset_to_roots() {
        let dir = make_dir("enter_reset");
        fs::write(dir.join("kick.wav"), b"x").unwrap();
        let parent = dir.parent().unwrap().to_path_buf();

        let mut b = SampleBrowser::new(vec![dir.clone()]);
        assert!(b.at_roots());

        // Enter the root
        assert!(b.enter_dir());
        assert!(!b.at_roots());
        assert_eq!(b.current_dir(), Some(dir.as_path()));

        // go_up navigates to parent of configured root (root escape)
        b.go_up();
        assert!(!b.at_roots());
        assert_eq!(b.current_dir(), Some(parent.as_path()));

        // reset_to_roots returns to roots list
        b.reset_to_roots();
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

        // go up returns to root dir
        b.go_up();
        assert_eq!(b.current_dir(), Some(dir.as_path()));

        // go up again escapes past the configured root boundary
        b.go_up();
        let expected_parent = dir.parent().unwrap();
        assert_eq!(b.current_dir(), Some(expected_parent));
        assert!(!b.at_roots());

        // reset_to_roots() returns to roots list
        b.reset_to_roots();
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

    // --- browser_title ---

    #[test]
    fn test_title_at_root_list() {
        let dir = make_dir("title_list");
        let b = SampleBrowser::new(vec![dir.clone()]);
        assert_eq!(browser_title(&b), " Sample Browser ");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_title_at_configured_root_no_double_slash() {
        let base = make_dir("title_root_base");
        let root = base.join("Music");
        fs::create_dir_all(&root).unwrap();

        let mut b = SampleBrowser::new(vec![root.clone()]);
        b.enter_dir();
        assert_eq!(b.current_dir(), Some(root.as_path()));

        let title = browser_title(&b);
        assert_eq!(
            title, " Sample Browser / Music ",
            "title at root: {title:?}"
        );
        assert!(!title.contains("/ /"), "no double-slash: {title:?}");

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn test_title_inside_root() {
        let base = make_dir("title_inside_base");
        let root = base.join("Sounds");
        let sub = root.join("kicks");
        fs::create_dir_all(&sub).unwrap();

        let mut b = SampleBrowser::new(vec![root.clone()]);
        b.enter_dir(); // → root
        b.enter_dir(); // → sub (first subdir is "kicks")

        let title = browser_title(&b);
        assert!(title.contains("Sounds/kicks"), "relative path: {title:?}");
        assert!(!title.contains("/ /"), "no double-slash: {title:?}");

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn test_title_above_root_no_double_slash() {
        let grandparent = make_dir("title_gp");
        let parent = grandparent.join("parent_dir");
        let root = parent.join("project");
        fs::create_dir_all(&root).unwrap();

        let mut b = SampleBrowser::new(vec![root.clone()]);
        b.enter_dir(); // → root
        b.go_up(); // → parent (above configured root)

        let title = browser_title(&b);
        assert!(
            !title.contains("/ /"),
            "no double-slash when above root: {title:?}"
        );
        assert!(title.contains("parent_dir"), "shows dir name: {title:?}");

        fs::remove_dir_all(&grandparent).ok();
    }

    // --- Bookmarks ---

    #[test]
    fn test_bookmarked_dir_shown_first() {
        let a = make_dir("bm_first_a");
        let b = make_dir("bm_first_b");

        let mut browser = SampleBrowser::new(vec![a.clone(), b.clone()]);
        browser.set_bookmarks(vec![b.clone()]);

        assert!(browser.at_roots());
        assert_eq!(browser.entries().len(), 2, "both dirs present");
        assert_eq!(
            browser.entries()[0].path,
            b,
            "bookmarked dir should be first"
        );
        assert!(browser.entries()[0].is_bookmarked);
        assert!(!browser.entries()[1].is_bookmarked);

        fs::remove_dir_all(&a).ok();
        fs::remove_dir_all(&b).ok();
    }

    #[test]
    fn test_bookmarked_entry_has_flag() {
        let a = make_dir("bm_flag_a");
        let b = make_dir("bm_flag_b");

        let mut browser = SampleBrowser::new(vec![a.clone(), b.clone()]);
        browser.set_bookmarks(vec![a.clone()]);

        let entry = browser.entries().iter().find(|e| e.path == a).unwrap();
        assert!(entry.is_bookmarked);

        let entry_b = browser.entries().iter().find(|e| e.path == b).unwrap();
        assert!(!entry_b.is_bookmarked);

        fs::remove_dir_all(&a).ok();
        fs::remove_dir_all(&b).ok();
    }

    #[test]
    fn test_in_dir_entries_never_bookmarked() {
        let dir = make_dir("bm_indir");
        let sub = dir.join("sub");
        fs::create_dir_all(&sub).unwrap();

        let mut browser = SampleBrowser::new(vec![dir.clone()]);
        // Even if sub is in bookmarks, entries inside a dir show is_bookmarked=false
        browser.set_bookmarks(vec![sub.clone()]);
        browser.enter_dir();

        for entry in browser.entries() {
            assert!(
                !entry.is_bookmarked,
                "entries inside a dir should not show is_bookmarked"
            );
        }

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_selected_is_dir_on_dir_entry() {
        let dir = make_dir("is_dir_sel");
        let sub = dir.join("sub");
        fs::create_dir_all(&sub).unwrap();

        let mut browser = SampleBrowser::new(vec![dir.clone()]);
        browser.enter_dir();

        assert!(browser.selected_is_dir());
        assert!(!browser.selected_is_file());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_selected_is_bookmarked_reflects_state() {
        let a = make_dir("bm_sel_a");
        let b = make_dir("bm_sel_b");

        let mut browser = SampleBrowser::new(vec![a.clone(), b.clone()]);
        browser.set_bookmarks(vec![a.clone()]);

        // After set_bookmarks, a is first
        browser.select(0); // a (bookmarked)
        assert!(browser.selected_is_bookmarked());

        browser.select(1); // b (not bookmarked)
        assert!(!browser.selected_is_bookmarked());

        fs::remove_dir_all(&a).ok();
        fs::remove_dir_all(&b).ok();
    }

    // --- Waveform cache API ---

    #[test]
    fn test_set_and_clear_waveform() {
        let dir = make_dir("wf_set_clear");
        let path = dir.join("kick.wav");
        let mut browser = SampleBrowser::new(vec![dir.clone()]);

        assert!(browser.waveform_peaks().is_empty());
        assert!(browser.waveform_path().is_none());

        browser.set_waveform_peaks(path.clone(), vec![0.1, 0.5, 0.8]);
        assert_eq!(browser.waveform_peaks().len(), 3);
        assert_eq!(browser.waveform_path(), Some(path.as_path()));

        browser.clear_waveform();
        assert!(browser.waveform_peaks().is_empty());
        assert!(browser.waveform_path().is_none());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_compute_waveform_peaks_nonexistent_file() {
        let peaks = compute_waveform_peaks(Path::new("/nonexistent/file.wav"), 64);
        assert!(peaks.is_empty(), "bad path should return empty peaks");
    }

    #[test]
    fn test_compute_waveform_peaks_valid_wav() {
        use std::io::Write;

        let dir = make_dir("wf_peaks_valid");
        let path = dir.join("test.wav");

        // Write a minimal PCM WAV: 44100 Hz, 1 ch, 16-bit, 0.1 s
        fn write_wav(p: &Path, sr: u32, ch: u16, samples: &[i16]) {
            let data_len = (samples.len() * 2) as u32;
            let mut f = std::fs::File::create(p).unwrap();
            f.write_all(b"RIFF").unwrap();
            f.write_all(&(36 + data_len).to_le_bytes()).unwrap();
            f.write_all(b"WAVE").unwrap();
            f.write_all(b"fmt ").unwrap();
            f.write_all(&16u32.to_le_bytes()).unwrap();
            f.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
            f.write_all(&ch.to_le_bytes()).unwrap();
            f.write_all(&sr.to_le_bytes()).unwrap();
            f.write_all(&(sr * ch as u32 * 2).to_le_bytes()).unwrap();
            f.write_all(&(ch * 2).to_le_bytes()).unwrap();
            f.write_all(&16u16.to_le_bytes()).unwrap();
            f.write_all(b"data").unwrap();
            f.write_all(&data_len.to_le_bytes()).unwrap();
            for &s in samples {
                f.write_all(&s.to_le_bytes()).unwrap();
            }
        }

        // 4410 mono samples at half amplitude (16383 ≈ 0.5 of i16::MAX)
        let samples: Vec<i16> = (0..4410).map(|_| 16383i16).collect();
        write_wav(&path, 44100, 1, &samples);

        let n = 32;
        let peaks = compute_waveform_peaks(&path, n);
        assert_eq!(peaks.len(), n, "should return exactly n_bars peaks");
        for &p in &peaks {
            assert!(p > 0.0 && p <= 1.0, "peak {p} should be in (0, 1]");
        }

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_compute_waveform_peaks_zero_bars() {
        let peaks = compute_waveform_peaks(Path::new("/some/file.wav"), 0);
        assert!(peaks.is_empty(), "n_bars=0 should return empty vec");
    }

    // --- format_preview_time ---

    #[test]
    fn test_format_preview_time_zero() {
        assert_eq!(format_preview_time(0.0), "00:00.00");
    }

    #[test]
    fn test_format_preview_time_one_second_half() {
        assert_eq!(format_preview_time(1.5), "00:01.50");
    }

    #[test]
    fn test_format_preview_time_one_minute() {
        assert_eq!(format_preview_time(60.0), "01:00.00");
    }

    #[test]
    fn test_format_preview_time_mixed() {
        // 1m 23.45s
        assert_eq!(format_preview_time(83.45), "01:23.45");
    }

    // --- cursor_col_for ---

    #[test]
    fn test_cursor_col_at_start() {
        assert_eq!(cursor_col_for(0, 1000, 40), 0);
    }

    #[test]
    fn test_cursor_col_at_end_clamped() {
        // pos == total: should clamp to w - 1
        assert_eq!(cursor_col_for(1000, 1000, 40), 39);
    }

    #[test]
    fn test_cursor_col_midpoint() {
        assert_eq!(cursor_col_for(500, 1000, 40), 20);
    }

    #[test]
    fn test_cursor_col_zero_total() {
        assert_eq!(cursor_col_for(42, 0, 40), 0);
    }

    #[test]
    fn test_cursor_col_zero_width() {
        assert_eq!(cursor_col_for(42, 1000, 0), 0);
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
