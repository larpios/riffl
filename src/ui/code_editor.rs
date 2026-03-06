/// Code editor panel for writing and executing Rhai DSL scripts.
///
/// Provides a text editor widget with line numbers, basic syntax highlighting,
/// cursor navigation, and a script output/error display area.
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::dsl::examples::TEMPLATES;

use super::theme::Theme;

/// Rhai keywords for syntax highlighting.
const KEYWORDS: &[&str] = &[
    "let", "if", "else", "for", "in", "while", "loop", "fn", "return", "true", "false", "break",
    "continue", "throw", "try", "catch",
];

/// The code editor state, managing text buffer, cursor, and output.
#[derive(Debug, Clone)]
pub struct CodeEditor {
    /// Lines of text in the editor buffer.
    lines: Vec<String>,
    /// Cursor row (0-indexed, into `lines`).
    cursor_row: usize,
    /// Cursor column (0-indexed, byte offset within the current line).
    cursor_col: usize,
    /// Vertical scroll offset (first visible line).
    scroll_offset: usize,
    /// Horizontal scroll offset (first visible column).
    h_scroll: usize,
    /// Output text from the last script execution (or error message).
    output: String,
    /// Whether the last output was an error.
    pub output_is_error: bool,
    /// Whether the editor is currently focused/active.
    pub active: bool,
    /// Whether the template menu overlay is visible.
    pub show_templates: bool,
    /// Currently highlighted template index in the menu.
    pub template_cursor: usize,
}

impl CodeEditor {
    /// Create a new empty code editor.
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
            h_scroll: 0,
            output: String::new(),
            output_is_error: false,
            active: false,
            show_templates: false,
            template_cursor: 0,
        }
    }

    // ── Accessors ──────────────────────────────────────────────

    /// Get the full text content of the editor as a single string.
    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    /// Get a reference to the lines.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Get the cursor row.
    pub fn cursor_row(&self) -> usize {
        self.cursor_row
    }

    /// Get the cursor column.
    pub fn cursor_col(&self) -> usize {
        self.cursor_col
    }

    /// Get the scroll offset.
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Get the output text.
    pub fn output(&self) -> &str {
        &self.output
    }

    /// Get number of lines.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    // ── Mutations ──────────────────────────────────────────────

    /// Set the editor content from a string (replaces everything).
    pub fn set_text(&mut self, text: &str) {
        self.lines = text.lines().map(|l| l.to_string()).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
        self.h_scroll = 0;
    }

    /// Set the output/result text.
    pub fn set_output(&mut self, text: String, is_error: bool) {
        self.output = text;
        self.output_is_error = is_error;
    }

    /// Clear the output area.
    pub fn clear_output(&mut self) {
        self.output.clear();
        self.output_is_error = false;
    }

    // ── Template menu ─────────────────────────────────────────

    /// Toggle the template menu overlay.
    pub fn toggle_templates(&mut self) {
        self.show_templates = !self.show_templates;
        if self.show_templates {
            self.template_cursor = 0;
        }
    }

    /// Move the template cursor up.
    pub fn template_up(&mut self) {
        if self.template_cursor > 0 {
            self.template_cursor -= 1;
        }
    }

    /// Move the template cursor down.
    pub fn template_down(&mut self) {
        if self.template_cursor + 1 < TEMPLATES.len() {
            self.template_cursor += 1;
        }
    }

    /// Load the currently selected template into the editor, closing the menu.
    pub fn load_selected_template(&mut self) {
        if let Some(t) = TEMPLATES.get(self.template_cursor) {
            self.set_text(t.code);
            self.show_templates = false;
        }
    }

    /// Close the template menu without loading anything.
    pub fn close_templates(&mut self) {
        self.show_templates = false;
    }

    // ── Cursor navigation ──────────────────────────────────────

    /// Move cursor left by one character.
    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            // Wrap to end of previous line
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
        }
        self.ensure_visible();
    }

    /// Move cursor right by one character.
    pub fn move_right(&mut self) {
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.lines.len() {
            // Wrap to start of next line
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
        self.ensure_visible();
    }

    /// Move cursor up by one line.
    pub fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.clamp_cursor_col();
        }
        self.ensure_visible();
    }

    /// Move cursor down by one line.
    pub fn move_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.clamp_cursor_col();
        }
        self.ensure_visible();
    }

    /// Move cursor to the beginning of the current line.
    pub fn move_home(&mut self) {
        self.cursor_col = 0;
        self.ensure_visible();
    }

    /// Move cursor to the end of the current line.
    pub fn move_end(&mut self) {
        self.cursor_col = self.lines[self.cursor_row].len();
        self.ensure_visible();
    }

    /// Move cursor up by a page (given visible height).
    pub fn page_up(&mut self, page_size: usize) {
        let jump = page_size.max(1);
        self.cursor_row = self.cursor_row.saturating_sub(jump);
        self.clamp_cursor_col();
        self.ensure_visible();
    }

    /// Move cursor down by a page (given visible height).
    pub fn page_down(&mut self, page_size: usize) {
        let jump = page_size.max(1);
        self.cursor_row = (self.cursor_row + jump).min(self.lines.len().saturating_sub(1));
        self.clamp_cursor_col();
        self.ensure_visible();
    }

    // ── Text editing ───────────────────────────────────────────

    /// Insert a character at the cursor position.
    pub fn insert_char(&mut self, ch: char) {
        let col = self.cursor_col.min(self.lines[self.cursor_row].len());
        self.lines[self.cursor_row].insert(col, ch);
        self.cursor_col = col + ch.len_utf8();
        self.ensure_visible();
    }

    /// Insert a newline at the cursor, splitting the current line.
    pub fn insert_newline(&mut self) {
        let col = self.cursor_col.min(self.lines[self.cursor_row].len());
        let rest = self.lines[self.cursor_row][col..].to_string();
        self.lines[self.cursor_row].truncate(col);
        self.cursor_row += 1;
        self.lines.insert(self.cursor_row, rest);
        self.cursor_col = 0;
        self.ensure_visible();
    }

    /// Delete the character before the cursor (Backspace).
    pub fn backspace(&mut self) {
        if self.cursor_col > 0 {
            let col = self.cursor_col.min(self.lines[self.cursor_row].len());
            if col > 0 {
                self.lines[self.cursor_row].remove(col - 1);
                self.cursor_col = col - 1;
            }
        } else if self.cursor_row > 0 {
            // Merge with previous line
            let current_line = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&current_line);
        }
        self.ensure_visible();
    }

    /// Delete the character at the cursor position (Delete key).
    pub fn delete(&mut self) {
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col < line_len {
            self.lines[self.cursor_row].remove(self.cursor_col);
        } else if self.cursor_row + 1 < self.lines.len() {
            // Merge next line into current
            let next_line = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next_line);
        }
    }

    // ── Internal helpers ───────────────────────────────────────

    /// Clamp cursor column to the current line length.
    fn clamp_cursor_col(&mut self) {
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }
    }

    /// Ensure the cursor is within the visible scroll window.
    fn ensure_visible(&mut self) {
        // Vertical scrolling — use a fixed estimate; render adapts dynamically
        self.ensure_visible_with_height(20);
    }

    /// Ensure cursor is visible for a given viewport height.
    pub fn ensure_visible_with_height(&mut self, visible_rows: usize) {
        if visible_rows == 0 {
            return;
        }
        if self.cursor_row < self.scroll_offset {
            self.scroll_offset = self.cursor_row;
        } else if self.cursor_row >= self.scroll_offset + visible_rows {
            self.scroll_offset = self.cursor_row + 1 - visible_rows;
        }
    }
}

impl Default for CodeEditor {
    fn default() -> Self {
        Self::new()
    }
}

// ── Syntax highlighting ────────────────────────────────────────

/// Token kinds for syntax highlighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Keyword,
    String,
    Number,
    Comment,
    Normal,
}

/// A syntax-highlighted span within a line.
#[derive(Debug, Clone)]
struct HighlightSpan {
    text: String,
    kind: TokenKind,
}

/// Tokenize a single line into highlighted spans.
fn highlight_line(line: &str) -> Vec<HighlightSpan> {
    let mut spans = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Comment: // to end of line
        if i + 1 < len && chars[i] == '/' && chars[i + 1] == '/' {
            let text: String = chars[i..].iter().collect();
            spans.push(HighlightSpan {
                text,
                kind: TokenKind::Comment,
            });
            break;
        }

        // String literal (double-quoted)
        if chars[i] == '"' {
            let start = i;
            i += 1;
            while i < len && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < len {
                    i += 1; // skip escaped char
                }
                i += 1;
            }
            if i < len {
                i += 1; // closing quote
            }
            let text: String = chars[start..i].iter().collect();
            spans.push(HighlightSpan {
                text,
                kind: TokenKind::String,
            });
            continue;
        }

        // Number literal
        if chars[i].is_ascii_digit()
            || (chars[i] == '-'
                && i + 1 < len
                && chars[i + 1].is_ascii_digit()
                && (i == 0 || !chars[i - 1].is_alphanumeric()))
        {
            let start = i;
            if chars[i] == '-' {
                i += 1;
            }
            while i < len && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            spans.push(HighlightSpan {
                text,
                kind: TokenKind::Number,
            });
            continue;
        }

        // Identifier / keyword
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let kind = if KEYWORDS.contains(&word.as_str()) {
                TokenKind::Keyword
            } else {
                TokenKind::Normal
            };
            spans.push(HighlightSpan { text: word, kind });
            continue;
        }

        // Whitespace / punctuation — collect contiguous normal chars
        let start = i;
        while i < len
            && !chars[i].is_alphanumeric()
            && chars[i] != '_'
            && chars[i] != '"'
            && !(chars[i] == '/' && i + 1 < len && chars[i + 1] == '/')
        {
            i += 1;
        }
        if i > start {
            let text: String = chars[start..i].iter().collect();
            spans.push(HighlightSpan {
                text,
                kind: TokenKind::Normal,
            });
        }
    }

    spans
}

/// Map a token kind to a ratatui Style.
fn token_style(kind: TokenKind, theme: &Theme) -> Style {
    match kind {
        TokenKind::Keyword => Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
        TokenKind::String => Style::default().fg(Color::Green),
        TokenKind::Number => Style::default().fg(Color::LightYellow),
        TokenKind::Comment => Style::default()
            .fg(theme.text_dimmed)
            .add_modifier(Modifier::ITALIC),
        TokenKind::Normal => Style::default().fg(theme.text),
    }
}

// ── Rendering ──────────────────────────────────────────────────

/// Render the code editor panel into the given area.
///
/// The area is split vertically: ~75% for the editor, ~25% for the output panel.
pub fn render_code_editor(frame: &mut Frame, area: Rect, editor: &CodeEditor, theme: &Theme) {
    // Split area into editor (top) and output (bottom)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(output_height(area.height)),
        ])
        .split(area);

    let editor_area = chunks[0];
    let output_area = chunks[1];

    render_editor_panel(frame, editor_area, editor, theme);
    render_output_panel(frame, output_area, editor, theme);

    // Render template menu overlay if active
    if editor.show_templates {
        render_template_menu(frame, area, editor, theme);
    }
}

/// Calculate a reasonable output panel height.
fn output_height(total: u16) -> u16 {
    // At least 3, at most 25% of total height
    let quarter = total / 4;
    quarter.max(3).min(8)
}

/// Render the text editor portion with line numbers and syntax highlighting.
fn render_editor_panel(frame: &mut Frame, area: Rect, editor: &CodeEditor, theme: &Theme) {
    let border_style = if editor.active {
        Style::default().fg(theme.border_focused)
    } else {
        theme.border_style()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(" Code Editor (Rhai) ")
        .title_alignment(Alignment::Left);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    // Line number gutter width: enough for the largest line number + space
    let max_line_num = editor.lines.len();
    let gutter_width = format!("{}", max_line_num).len() as u16 + 2; // " N " padding
    let _code_width = inner.width.saturating_sub(gutter_width);

    let visible_rows = inner.height as usize;

    // Dynamically update scroll offset based on actual visible rows
    let scroll_offset = {
        let mut so = editor.scroll_offset;
        if editor.cursor_row < so {
            so = editor.cursor_row;
        } else if editor.cursor_row >= so + visible_rows {
            so = editor.cursor_row + 1 - visible_rows;
        }
        so
    };

    let mut lines: Vec<Line> = Vec::with_capacity(visible_rows);

    for display_idx in 0..visible_rows {
        let line_idx = scroll_offset + display_idx;
        if line_idx >= editor.lines.len() {
            // Tilde for lines beyond the buffer (like vim)
            let mut spans = Vec::new();
            let gutter = format!("{:>width$} ", "~", width = gutter_width as usize - 1);
            spans.push(Span::styled(gutter, Style::default().fg(theme.text_dimmed)));
            lines.push(Line::from(spans));
            continue;
        }

        let mut spans = Vec::new();

        // Line number gutter
        let line_num = line_idx + 1;
        let gutter_text = format!("{:>width$} ", line_num, width = gutter_width as usize - 1);
        let gutter_style = if line_idx == editor.cursor_row {
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_dimmed)
        };
        spans.push(Span::styled(gutter_text, gutter_style));

        // Syntax-highlighted code with cursor overlay
        let source_line = &editor.lines[line_idx];
        let is_cursor_line = line_idx == editor.cursor_row && editor.active;

        if is_cursor_line {
            // Render character-by-character for precise cursor placement
            let hl_spans = highlight_line(source_line);
            let mut col = 0usize;
            for hs in &hl_spans {
                let style = token_style(hs.kind, theme);
                for ch in hs.text.chars() {
                    if col == editor.cursor_col {
                        // Cursor character
                        spans.push(Span::styled(
                            ch.to_string(),
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::LightYellow)
                                .add_modifier(Modifier::BOLD),
                        ));
                    } else {
                        spans.push(Span::styled(ch.to_string(), style));
                    }
                    col += 1;
                }
            }
            // If cursor is at end of line, show block cursor on a space
            if editor.cursor_col >= source_line.len() {
                spans.push(Span::styled(
                    " ",
                    Style::default().fg(Color::Black).bg(Color::LightYellow),
                ));
            }
        } else {
            // Normal line — batch highlighted spans
            let hl_spans = highlight_line(source_line);
            for hs in &hl_spans {
                spans.push(Span::styled(hs.text.clone(), token_style(hs.kind, theme)));
            }
        }

        lines.push(Line::from(spans));
    }

    let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
    frame.render_widget(paragraph, inner);
}

/// Render the output/error panel below the editor.
fn render_output_panel(frame: &mut Frame, area: Rect, editor: &CodeEditor, theme: &Theme) {
    let border_color = if editor.output_is_error {
        theme.error_color()
    } else if !editor.output.is_empty() {
        theme.success_color()
    } else {
        theme.border_color()
    };

    let title = if editor.output_is_error {
        " Script Error "
    } else if !editor.output.is_empty() {
        " Script Output "
    } else {
        " Output "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title)
        .title_alignment(Alignment::Left);

    let text_style = if editor.output_is_error {
        Style::default().fg(theme.error_color())
    } else {
        Style::default().fg(theme.text)
    };

    let content = if editor.output.is_empty() {
        "Press Ctrl+Enter to execute script".to_string()
    } else {
        editor.output.clone()
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .style(text_style)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Render the template menu as a centered overlay.
fn render_template_menu(frame: &mut Frame, area: Rect, editor: &CodeEditor, theme: &Theme) {
    // Calculate centered overlay area
    let menu_width = 50u16.min(area.width.saturating_sub(4));
    let menu_height = (TEMPLATES.len() as u16 * 2 + 3).min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(menu_width)) / 2;
    let y = area.y + (area.height.saturating_sub(menu_height)) / 2;
    let menu_area = Rect::new(x, y, menu_width, menu_height);

    // Clear background
    let clear = Paragraph::new("")
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().bg(Color::Black));
    frame.render_widget(clear, menu_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.primary))
        .title(" Templates (↑↓ Enter Esc) ")
        .title_alignment(Alignment::Center);

    let inner = block.inner(menu_area);
    frame.render_widget(block, menu_area);

    // Build menu items
    let mut lines: Vec<Line> = Vec::new();
    for (i, t) in TEMPLATES.iter().enumerate() {
        let is_selected = i == editor.template_cursor;
        let name_style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(theme.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD)
        };
        let desc_style = if is_selected {
            Style::default().fg(Color::Black).bg(theme.primary)
        } else {
            Style::default().fg(theme.text_dimmed)
        };

        let prefix = if is_selected { "▸ " } else { "  " };
        lines.push(Line::from(vec![
            Span::styled(prefix, name_style),
            Span::styled(t.name, name_style),
        ]));
        lines.push(Line::from(vec![
            Span::styled("    ", desc_style),
            Span::styled(t.description, desc_style),
        ]));
    }

    let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
    frame.render_widget(paragraph, inner);
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_editor_has_one_empty_line() {
        let ed = CodeEditor::new();
        assert_eq!(ed.line_count(), 1);
        assert_eq!(ed.lines()[0], "");
        assert_eq!(ed.cursor_row(), 0);
        assert_eq!(ed.cursor_col(), 0);
    }

    #[test]
    fn test_set_text_replaces_content() {
        let mut ed = CodeEditor::new();
        ed.set_text("hello\nworld");
        assert_eq!(ed.line_count(), 2);
        assert_eq!(ed.lines()[0], "hello");
        assert_eq!(ed.lines()[1], "world");
        assert_eq!(ed.cursor_row(), 0);
        assert_eq!(ed.cursor_col(), 0);
    }

    #[test]
    fn test_set_text_empty_gives_one_line() {
        let mut ed = CodeEditor::new();
        ed.set_text("");
        assert_eq!(ed.line_count(), 1);
    }

    #[test]
    fn test_text_round_trip() {
        let mut ed = CodeEditor::new();
        let original = "line1\nline2\nline3";
        ed.set_text(original);
        assert_eq!(ed.text(), original);
    }

    #[test]
    fn test_insert_char() {
        let mut ed = CodeEditor::new();
        ed.insert_char('a');
        ed.insert_char('b');
        ed.insert_char('c');
        assert_eq!(ed.lines()[0], "abc");
        assert_eq!(ed.cursor_col(), 3);
    }

    #[test]
    fn test_insert_newline() {
        let mut ed = CodeEditor::new();
        ed.set_text("hello world");
        ed.cursor_col = 5;
        ed.insert_newline();
        assert_eq!(ed.line_count(), 2);
        assert_eq!(ed.lines()[0], "hello");
        assert_eq!(ed.lines()[1], " world");
        assert_eq!(ed.cursor_row(), 1);
        assert_eq!(ed.cursor_col(), 0);
    }

    #[test]
    fn test_backspace_within_line() {
        let mut ed = CodeEditor::new();
        ed.set_text("abc");
        ed.cursor_col = 3;
        ed.backspace();
        assert_eq!(ed.lines()[0], "ab");
        assert_eq!(ed.cursor_col(), 2);
    }

    #[test]
    fn test_backspace_merges_lines() {
        let mut ed = CodeEditor::new();
        ed.set_text("hello\nworld");
        ed.cursor_row = 1;
        ed.cursor_col = 0;
        ed.backspace();
        assert_eq!(ed.line_count(), 1);
        assert_eq!(ed.lines()[0], "helloworld");
        assert_eq!(ed.cursor_row(), 0);
        assert_eq!(ed.cursor_col(), 5);
    }

    #[test]
    fn test_backspace_at_start_of_buffer() {
        let mut ed = CodeEditor::new();
        ed.set_text("hello");
        ed.cursor_col = 0;
        ed.backspace();
        // Should be a no-op
        assert_eq!(ed.lines()[0], "hello");
    }

    #[test]
    fn test_delete_within_line() {
        let mut ed = CodeEditor::new();
        ed.set_text("abc");
        ed.cursor_col = 1;
        ed.delete();
        assert_eq!(ed.lines()[0], "ac");
    }

    #[test]
    fn test_delete_merges_next_line() {
        let mut ed = CodeEditor::new();
        ed.set_text("hello\nworld");
        ed.cursor_col = 5; // end of "hello"
        ed.delete();
        assert_eq!(ed.line_count(), 1);
        assert_eq!(ed.lines()[0], "helloworld");
    }

    #[test]
    fn test_move_left_wraps_to_previous_line() {
        let mut ed = CodeEditor::new();
        ed.set_text("abc\ndef");
        ed.cursor_row = 1;
        ed.cursor_col = 0;
        ed.move_left();
        assert_eq!(ed.cursor_row(), 0);
        assert_eq!(ed.cursor_col(), 3);
    }

    #[test]
    fn test_move_right_wraps_to_next_line() {
        let mut ed = CodeEditor::new();
        ed.set_text("abc\ndef");
        ed.cursor_row = 0;
        ed.cursor_col = 3;
        ed.move_right();
        assert_eq!(ed.cursor_row(), 1);
        assert_eq!(ed.cursor_col(), 0);
    }

    #[test]
    fn test_move_up_clamps_col() {
        let mut ed = CodeEditor::new();
        ed.set_text("ab\nabcdef");
        ed.cursor_row = 1;
        ed.cursor_col = 5;
        ed.move_up();
        assert_eq!(ed.cursor_row(), 0);
        assert_eq!(ed.cursor_col(), 2); // clamped to "ab".len()
    }

    #[test]
    fn test_move_down_clamps_col() {
        let mut ed = CodeEditor::new();
        ed.set_text("abcdef\nab");
        ed.cursor_row = 0;
        ed.cursor_col = 5;
        ed.move_down();
        assert_eq!(ed.cursor_row(), 1);
        assert_eq!(ed.cursor_col(), 2);
    }

    #[test]
    fn test_home_and_end() {
        let mut ed = CodeEditor::new();
        ed.set_text("hello world");
        ed.cursor_col = 5;
        ed.move_home();
        assert_eq!(ed.cursor_col(), 0);
        ed.move_end();
        assert_eq!(ed.cursor_col(), 11);
    }

    #[test]
    fn test_page_up_down() {
        let mut ed = CodeEditor::new();
        let text = (0..50)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        ed.set_text(&text);
        ed.cursor_row = 25;

        ed.page_up(10);
        assert_eq!(ed.cursor_row(), 15);

        ed.page_down(10);
        assert_eq!(ed.cursor_row(), 25);

        // Page down past end
        ed.page_down(100);
        assert_eq!(ed.cursor_row(), 49);

        // Page up past start
        ed.page_up(100);
        assert_eq!(ed.cursor_row(), 0);
    }

    #[test]
    fn test_scroll_offset_follows_cursor() {
        let mut ed = CodeEditor::new();
        let text = (0..50)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        ed.set_text(&text);
        ed.cursor_row = 0;
        ed.scroll_offset = 0;

        // Move cursor beyond visible area
        for _ in 0..25 {
            ed.move_down();
        }
        // Scroll offset should have adjusted
        assert!(ed.scroll_offset > 0);
    }

    #[test]
    fn test_output_set_and_clear() {
        let mut ed = CodeEditor::new();
        assert!(ed.output().is_empty());

        ed.set_output("result: 42".to_string(), false);
        assert_eq!(ed.output(), "result: 42");
        assert!(!ed.output_is_error);

        ed.set_output("Error: syntax error at line 1".to_string(), true);
        assert!(ed.output_is_error);

        ed.clear_output();
        assert!(ed.output().is_empty());
        assert!(!ed.output_is_error);
    }

    // ── Syntax highlighting tests ──────────────────────────────

    #[test]
    fn test_highlight_keyword() {
        let spans = highlight_line("let x = 5;");
        assert!(spans
            .iter()
            .any(|s| s.kind == TokenKind::Keyword && s.text == "let"));
    }

    #[test]
    fn test_highlight_string() {
        let spans = highlight_line("let s = \"hello\";");
        assert!(spans
            .iter()
            .any(|s| s.kind == TokenKind::String && s.text == "\"hello\""));
    }

    #[test]
    fn test_highlight_number() {
        let spans = highlight_line("let x = 42;");
        assert!(spans
            .iter()
            .any(|s| s.kind == TokenKind::Number && s.text == "42"));
    }

    #[test]
    fn test_highlight_comment() {
        let spans = highlight_line("let x = 5; // a comment");
        assert!(spans
            .iter()
            .any(|s| s.kind == TokenKind::Comment && s.text.contains("// a comment")));
    }

    #[test]
    fn test_highlight_empty_line() {
        let spans = highlight_line("");
        assert!(spans.is_empty());
    }

    #[test]
    fn test_highlight_all_keywords() {
        for kw in KEYWORDS {
            let spans = highlight_line(kw);
            assert!(
                spans
                    .iter()
                    .any(|s| s.kind == TokenKind::Keyword && s.text == *kw),
                "keyword '{}' not highlighted",
                kw
            );
        }
    }

    #[test]
    fn test_highlight_float_number() {
        let spans = highlight_line("3.14");
        assert!(spans
            .iter()
            .any(|s| s.kind == TokenKind::Number && s.text == "3.14"));
    }

    #[test]
    fn test_highlight_escaped_string() {
        let spans = highlight_line("\"he\\\"llo\"");
        assert!(spans.iter().any(|s| s.kind == TokenKind::String));
    }

    #[test]
    fn test_highlight_identifier_not_keyword() {
        let spans = highlight_line("letter");
        // "letter" starts with "let" but should NOT be a keyword
        assert!(spans.iter().all(|s| s.kind != TokenKind::Keyword));
    }

    #[test]
    fn test_highlight_multiple_keywords() {
        let spans = highlight_line("if true { return false; }");
        let kw_count = spans
            .iter()
            .filter(|s| s.kind == TokenKind::Keyword)
            .count();
        // if, true, return, false = 4 keywords
        assert_eq!(kw_count, 4);
    }

    // ── Edge cases ─────────────────────────────────────────────

    #[test]
    fn test_insert_char_at_start() {
        let mut ed = CodeEditor::new();
        ed.set_text("hello");
        ed.cursor_col = 0;
        ed.insert_char('X');
        assert_eq!(ed.lines()[0], "Xhello");
    }

    #[test]
    fn test_insert_char_in_middle() {
        let mut ed = CodeEditor::new();
        ed.set_text("hllo");
        ed.cursor_col = 1;
        ed.insert_char('e');
        assert_eq!(ed.lines()[0], "hello");
    }

    #[test]
    fn test_delete_at_end_of_buffer() {
        let mut ed = CodeEditor::new();
        ed.set_text("abc");
        ed.cursor_col = 3;
        ed.delete(); // no-op
        assert_eq!(ed.lines()[0], "abc");
    }

    #[test]
    fn test_move_up_at_top() {
        let mut ed = CodeEditor::new();
        ed.set_text("abc");
        ed.move_up(); // no-op
        assert_eq!(ed.cursor_row(), 0);
    }

    #[test]
    fn test_move_down_at_bottom() {
        let mut ed = CodeEditor::new();
        ed.set_text("abc");
        ed.move_down(); // no-op
        assert_eq!(ed.cursor_row(), 0);
    }

    #[test]
    fn test_move_left_at_buffer_start() {
        let mut ed = CodeEditor::new();
        ed.move_left(); // no-op
        assert_eq!(ed.cursor_row(), 0);
        assert_eq!(ed.cursor_col(), 0);
    }

    #[test]
    fn test_move_right_at_buffer_end() {
        let mut ed = CodeEditor::new();
        ed.set_text("abc");
        ed.cursor_col = 3;
        ed.move_right(); // no-op (single line, at end)
        assert_eq!(ed.cursor_row(), 0);
        assert_eq!(ed.cursor_col(), 3);
    }

    #[test]
    fn test_ensure_visible_with_height() {
        let mut ed = CodeEditor::new();
        let text = (0..100)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        ed.set_text(&text);
        ed.cursor_row = 50;
        ed.scroll_offset = 0;
        ed.ensure_visible_with_height(10);
        // Cursor at 50, visible 10 lines, so scroll should bring cursor into view
        assert!(ed.scroll_offset <= 50);
        assert!(ed.scroll_offset + 10 > 50);
    }

    #[test]
    fn test_default_is_same_as_new() {
        let ed1 = CodeEditor::new();
        let ed2 = CodeEditor::default();
        assert_eq!(ed1.line_count(), ed2.line_count());
        assert_eq!(ed1.cursor_row(), ed2.cursor_row());
        assert_eq!(ed1.cursor_col(), ed2.cursor_col());
    }

    // ── Template menu tests ──────────────────────────────────

    #[test]
    fn test_template_menu_initially_hidden() {
        let ed = CodeEditor::new();
        assert!(!ed.show_templates);
        assert_eq!(ed.template_cursor, 0);
    }

    #[test]
    fn test_toggle_templates_opens_menu() {
        let mut ed = CodeEditor::new();
        ed.toggle_templates();
        assert!(ed.show_templates);
        assert_eq!(ed.template_cursor, 0);
    }

    #[test]
    fn test_toggle_templates_closes_menu() {
        let mut ed = CodeEditor::new();
        ed.toggle_templates();
        ed.toggle_templates();
        assert!(!ed.show_templates);
    }

    #[test]
    fn test_toggle_templates_resets_cursor() {
        let mut ed = CodeEditor::new();
        ed.toggle_templates();
        ed.template_down();
        ed.template_down();
        assert_eq!(ed.template_cursor, 2);
        // Close and reopen — cursor should reset
        ed.toggle_templates();
        ed.toggle_templates();
        assert_eq!(ed.template_cursor, 0);
    }

    #[test]
    fn test_template_up_down_navigation() {
        let mut ed = CodeEditor::new();
        ed.toggle_templates();

        // Move down through all templates
        ed.template_down();
        assert_eq!(ed.template_cursor, 1);
        ed.template_down();
        assert_eq!(ed.template_cursor, 2);
        ed.template_down();
        assert_eq!(ed.template_cursor, 3);
        // Can't go past last
        ed.template_down();
        assert_eq!(ed.template_cursor, 3);

        // Move back up
        ed.template_up();
        assert_eq!(ed.template_cursor, 2);
        ed.template_up();
        assert_eq!(ed.template_cursor, 1);
        ed.template_up();
        assert_eq!(ed.template_cursor, 0);
        // Can't go past first
        ed.template_up();
        assert_eq!(ed.template_cursor, 0);
    }

    #[test]
    fn test_load_selected_template() {
        let mut ed = CodeEditor::new();
        ed.toggle_templates();
        ed.load_selected_template();
        // Menu should close
        assert!(!ed.show_templates);
        // Content should be loaded (first template: Simple Beat)
        let text = ed.text();
        assert!(text.contains("euclidean"), "Expected Simple Beat template");
    }

    #[test]
    fn test_load_second_template() {
        let mut ed = CodeEditor::new();
        ed.toggle_templates();
        ed.template_down();
        ed.load_selected_template();
        assert!(!ed.show_templates);
        let text = ed.text();
        assert!(
            text.contains("pentatonic"),
            "Expected Random Melody template"
        );
    }

    #[test]
    fn test_load_third_template() {
        let mut ed = CodeEditor::new();
        ed.toggle_templates();
        ed.template_down();
        ed.template_down();
        ed.load_selected_template();
        assert!(!ed.show_templates);
        let text = ed.text();
        assert!(text.contains("chord"), "Expected Arpeggiator template");
    }

    #[test]
    fn test_load_fourth_template() {
        let mut ed = CodeEditor::new();
        ed.toggle_templates();
        ed.template_down();
        ed.template_down();
        ed.template_down();
        ed.load_selected_template();
        assert!(!ed.show_templates);
        let text = ed.text();
        assert!(
            text.contains("Probability"),
            "Expected Probability Beat template"
        );
    }

    #[test]
    fn test_close_templates_without_loading() {
        let mut ed = CodeEditor::new();
        ed.set_text("existing code");
        ed.toggle_templates();
        ed.close_templates();
        assert!(!ed.show_templates);
        // Original content should be preserved
        assert_eq!(ed.text(), "existing code");
    }

    #[test]
    fn test_loading_template_resets_cursor_position() {
        let mut ed = CodeEditor::new();
        ed.set_text("line1\nline2\nline3");
        ed.cursor_row = 2;
        ed.cursor_col = 3;
        ed.toggle_templates();
        ed.load_selected_template();
        // set_text resets cursor to 0, 0
        assert_eq!(ed.cursor_row(), 0);
        assert_eq!(ed.cursor_col(), 0);
    }
}
