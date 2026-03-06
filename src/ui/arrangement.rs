/// Arrangement view UI for the song sequencer.
///
/// Displays the song's pattern sequence vertically, allowing navigation
/// and manipulation of the arrangement (pattern order).
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::pattern::note::NoteEvent;
use crate::song::Song;
use crate::ui::theme::Theme;

/// State for the arrangement view.
#[derive(Debug)]
pub struct ArrangementView {
    /// Currently selected position in the arrangement.
    cursor: usize,
}

impl ArrangementView {
    /// Create a new arrangement view.
    pub fn new() -> Self {
        Self { cursor: 0 }
    }

    /// Get the current cursor position.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Move cursor up in the arrangement.
    pub fn move_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Move cursor down in the arrangement.
    pub fn move_down(&mut self, arrangement_len: usize) {
        if arrangement_len > 0 && self.cursor < arrangement_len - 1 {
            self.cursor += 1;
        }
    }

    /// Append a pattern to the arrangement (after cursor position).
    /// Returns true if successful.
    pub fn append_pattern(&mut self, song: &mut Song, pattern_index: usize) -> bool {
        let pos = (self.cursor + 1).min(song.arrangement.len());
        if song.insert_in_arrangement(pos, pattern_index) {
            self.cursor = pos;
            true
        } else {
            false
        }
    }

    /// Remove the entry at the current cursor position.
    /// Returns true if successful.
    pub fn remove_at_cursor(&mut self, song: &mut Song) -> bool {
        if song.arrangement.len() <= 1 {
            return false; // Don't remove the last entry
        }
        if song.remove_from_arrangement(self.cursor).is_some() {
            if self.cursor >= song.arrangement.len() {
                self.cursor = song.arrangement.len().saturating_sub(1);
            }
            true
        } else {
            false
        }
    }

    /// Create a new empty pattern and append it to the arrangement.
    /// Returns the new pattern index if successful.
    pub fn create_new_pattern(&mut self, song: &mut Song) -> Option<usize> {
        let num_channels = if let Some(first) = song.patterns.first() {
            first.num_channels()
        } else {
            4
        };
        let new_pattern = crate::pattern::Pattern::new(64, num_channels);
        if let Some(idx) = song.add_pattern(new_pattern) {
            let pos = (self.cursor + 1).min(song.arrangement.len());
            song.insert_in_arrangement(pos, idx);
            self.cursor = pos;
            Some(idx)
        } else {
            None
        }
    }

    /// Clamp cursor to valid range (call after external arrangement changes).
    pub fn clamp_cursor(&mut self, arrangement_len: usize) {
        if arrangement_len == 0 {
            self.cursor = 0;
        } else if self.cursor >= arrangement_len {
            self.cursor = arrangement_len - 1;
        }
    }
}

impl Default for ArrangementView {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a short preview string of the first few notes in a pattern.
fn pattern_preview(pattern: &crate::pattern::Pattern) -> String {
    let mut notes = Vec::new();
    let max_preview = 4;
    for row in 0..pattern.num_rows() {
        if notes.len() >= max_preview {
            break;
        }
        for ch in 0..pattern.num_channels() {
            if notes.len() >= max_preview {
                break;
            }
            if let Some(cell) = pattern.get_cell(row, ch) {
                match &cell.note {
                    Some(NoteEvent::On(note)) => {
                        notes.push(note.display_str());
                    }
                    Some(NoteEvent::Off) => {
                        notes.push("===".to_string());
                    }
                    None => {}
                }
            }
        }
    }
    if notes.is_empty() {
        "(empty)".to_string()
    } else {
        notes.join(" ")
    }
}

/// Render the arrangement view.
pub fn render_arrangement(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    song: &Song,
    view: &ArrangementView,
    playback_position: Option<usize>,
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .title(" Arrangement ")
        .title_alignment(Alignment::Left);

    let inner = block.inner(area);
    let visible_rows = inner.height as usize;

    // Calculate scroll offset to keep cursor visible
    let total = song.arrangement.len();
    let scroll_offset = calculate_scroll(view.cursor, visible_rows, total);

    let mut lines: Vec<Line> = Vec::new();

    // Header line
    lines.push(Line::from(vec![Span::styled(
        format!("  {:>3}  {:>3}  {:>4}  {}", "Pos", "Pat", "Rows", "Preview"),
        Style::default().fg(theme.text_secondary),
    )]));

    let data_rows = visible_rows.saturating_sub(1); // reserve header
    for display_idx in 0..data_rows {
        let arr_idx = scroll_offset + display_idx;
        if arr_idx >= total {
            break;
        }

        let pattern_index = song.arrangement[arr_idx];
        let pattern = song.patterns.get(pattern_index);
        let num_rows = pattern.map_or(0, |p| p.num_rows());
        let preview = pattern.map_or_else(|| "???".to_string(), pattern_preview);

        let is_cursor = arr_idx == view.cursor;
        let is_playback = playback_position == Some(arr_idx);

        let pos_str = format!("  {:3}", arr_idx);
        let pat_str = format!("  {:02X}", pattern_index);
        let rows_str = format!("  {:4}", num_rows);
        let preview_str = format!("  {}", preview);

        let style = if is_cursor && is_playback {
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD)
        } else if is_cursor {
            theme.highlight_style()
        } else if is_playback {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };

        lines.push(Line::from(vec![
            Span::styled(pos_str, style),
            Span::styled(pat_str, style),
            Span::styled(rows_str, style),
            Span::styled(preview_str, style),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

/// Calculate scroll offset for arrangement list.
fn calculate_scroll(cursor: usize, visible_rows: usize, total: usize) -> usize {
    if visible_rows >= total {
        return 0;
    }
    if cursor < visible_rows / 2 {
        0
    } else if cursor + visible_rows / 2 >= total {
        total.saturating_sub(visible_rows)
    } else {
        cursor.saturating_sub(visible_rows / 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::note::{Note, Pitch};
    use crate::pattern::Pattern;

    fn test_song() -> Song {
        let mut song = Song::new("Test", 120.0);
        // Add a second pattern
        let mut p1 = Pattern::new(16, 4);
        p1.set_note(0, 0, Note::simple(Pitch::C, 4));
        p1.set_note(4, 0, Note::simple(Pitch::E, 4));
        song.add_pattern(p1);
        // Arrangement: [0, 1, 0]
        song.arrangement = vec![0, 1, 0];
        song
    }

    // --- ArrangementView basic state ---

    #[test]
    fn test_new_arrangement_view() {
        let view = ArrangementView::new();
        assert_eq!(view.cursor(), 0);
    }

    #[test]
    fn test_move_up_at_top() {
        let mut view = ArrangementView::new();
        view.move_up();
        assert_eq!(view.cursor(), 0);
    }

    #[test]
    fn test_move_down() {
        let mut view = ArrangementView::new();
        view.move_down(5);
        assert_eq!(view.cursor(), 1);
        view.move_down(5);
        assert_eq!(view.cursor(), 2);
    }

    #[test]
    fn test_move_down_at_bottom() {
        let mut view = ArrangementView::new();
        view.cursor = 4;
        view.move_down(5);
        assert_eq!(view.cursor(), 4); // can't go past len-1
    }

    #[test]
    fn test_move_down_empty_arrangement() {
        let mut view = ArrangementView::new();
        view.move_down(0);
        assert_eq!(view.cursor(), 0);
    }

    #[test]
    fn test_move_up_from_middle() {
        let mut view = ArrangementView::new();
        view.cursor = 3;
        view.move_up();
        assert_eq!(view.cursor(), 2);
    }

    // --- Append pattern ---

    #[test]
    fn test_append_pattern() {
        let mut song = test_song();
        let mut view = ArrangementView::new();
        view.cursor = 0;

        assert!(view.append_pattern(&mut song, 1));
        assert_eq!(song.arrangement, vec![0, 1, 1, 0]);
        assert_eq!(view.cursor(), 1);
    }

    #[test]
    fn test_append_pattern_invalid_index() {
        let mut song = test_song();
        let mut view = ArrangementView::new();

        // Pattern 99 doesn't exist
        assert!(!view.append_pattern(&mut song, 99));
        assert_eq!(song.arrangement.len(), 3); // unchanged
    }

    // --- Remove at cursor ---

    #[test]
    fn test_remove_at_cursor() {
        let mut song = test_song();
        let mut view = ArrangementView::new();
        view.cursor = 1;

        assert!(view.remove_at_cursor(&mut song));
        assert_eq!(song.arrangement, vec![0, 0]);
        assert_eq!(view.cursor(), 1);
    }

    #[test]
    fn test_remove_at_cursor_last_entry() {
        let mut song = Song::new("Test", 120.0);
        // Song starts with arrangement [0], can't remove the last one
        let mut view = ArrangementView::new();

        assert!(!view.remove_at_cursor(&mut song));
        assert_eq!(song.arrangement.len(), 1);
    }

    #[test]
    fn test_remove_at_cursor_clamps() {
        let mut song = test_song();
        let mut view = ArrangementView::new();
        view.cursor = 2; // last position

        assert!(view.remove_at_cursor(&mut song));
        // cursor should clamp to new last position
        assert_eq!(view.cursor(), 1);
    }

    // --- Create new pattern ---

    #[test]
    fn test_create_new_pattern() {
        let mut song = test_song();
        let initial_patterns = song.patterns.len();
        let mut view = ArrangementView::new();
        view.cursor = 0;

        let idx = view.create_new_pattern(&mut song);
        assert!(idx.is_some());
        assert_eq!(song.patterns.len(), initial_patterns + 1);
        // New pattern should be in arrangement after cursor
        assert_eq!(view.cursor(), 1);
        assert_eq!(song.arrangement[1], idx.unwrap());
    }

    // --- Clamp cursor ---

    #[test]
    fn test_clamp_cursor() {
        let mut view = ArrangementView::new();
        view.cursor = 10;
        view.clamp_cursor(3);
        assert_eq!(view.cursor(), 2);
    }

    #[test]
    fn test_clamp_cursor_empty() {
        let mut view = ArrangementView::new();
        view.cursor = 5;
        view.clamp_cursor(0);
        assert_eq!(view.cursor(), 0);
    }

    #[test]
    fn test_clamp_cursor_in_range() {
        let mut view = ArrangementView::new();
        view.cursor = 2;
        view.clamp_cursor(5);
        assert_eq!(view.cursor(), 2); // no change
    }

    // --- Scroll calculation ---

    #[test]
    fn test_scroll_fits_in_view() {
        assert_eq!(calculate_scroll(0, 20, 5), 0);
        assert_eq!(calculate_scroll(4, 20, 5), 0);
    }

    #[test]
    fn test_scroll_at_top() {
        assert_eq!(calculate_scroll(0, 10, 50), 0);
        assert_eq!(calculate_scroll(3, 10, 50), 0);
    }

    #[test]
    fn test_scroll_middle() {
        assert_eq!(calculate_scroll(25, 10, 50), 20);
    }

    #[test]
    fn test_scroll_at_bottom() {
        assert_eq!(calculate_scroll(49, 10, 50), 40);
    }

    // --- Pattern preview ---

    #[test]
    fn test_pattern_preview_empty() {
        let pattern = Pattern::new(16, 4);
        assert_eq!(pattern_preview(&pattern), "(empty)");
    }

    #[test]
    fn test_pattern_preview_with_notes() {
        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::simple(Pitch::C, 4));
        pattern.set_note(4, 0, Note::simple(Pitch::E, 4));
        pattern.set_note(8, 0, Note::simple(Pitch::G, 4));

        let preview = pattern_preview(&pattern);
        assert!(preview.contains("C-4"));
        assert!(preview.contains("E-4"));
        assert!(preview.contains("G-4"));
    }

    #[test]
    fn test_pattern_preview_max_four() {
        let mut pattern = Pattern::new(16, 4);
        for row in 0..8 {
            pattern.set_note(row, 0, Note::simple(Pitch::C, 4));
        }

        let preview = pattern_preview(&pattern);
        // Should only show 4 notes max
        let count = preview.matches("C-4").count();
        assert_eq!(count, 4);
    }

    // --- Default trait ---

    #[test]
    fn test_arrangement_view_default() {
        let view = ArrangementView::default();
        assert_eq!(view.cursor(), 0);
    }
}
