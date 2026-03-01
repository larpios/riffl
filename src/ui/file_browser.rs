//! File browser modal for loading audio samples
//!
//! Provides an interactive file list filtered to supported audio formats
//! (.wav, .flac, .ogg). Users navigate with j/k keys and select with Enter.

use std::path::{Path, PathBuf};

/// Supported audio file extensions for the sample browser.
const AUDIO_EXTENSIONS: &[&str] = &["wav", "flac", "ogg"];

/// Interactive file browser state for selecting audio samples.
#[derive(Debug, Clone)]
pub struct FileBrowser {
    /// Current directory being browsed
    directory: PathBuf,
    /// List of audio files found in the directory
    entries: Vec<PathBuf>,
    /// Currently selected index in the file list
    selected: usize,
    /// Whether the browser is currently open/active
    pub active: bool,
}

impl FileBrowser {
    /// Create a new file browser rooted at the given directory.
    ///
    /// Scans the directory for supported audio files (.wav, .flac, .ogg).
    pub fn new(directory: &Path) -> Self {
        let entries = scan_audio_files(directory);
        Self {
            directory: directory.to_path_buf(),
            entries,
            selected: 0,
            active: false,
        }
    }

    /// Open the file browser (set active and refresh file list).
    pub fn open(&mut self) {
        self.entries = scan_audio_files(&self.directory);
        self.selected = 0;
        self.active = true;
    }

    /// Open the file browser with a specific directory.
    pub fn open_dir(&mut self, directory: &Path) {
        self.directory = directory.to_path_buf();
        self.open();
    }

    /// Close the file browser.
    pub fn close(&mut self) {
        self.active = false;
    }

    /// Move selection up by one entry.
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down by one entry.
    pub fn move_down(&mut self) {
        if !self.entries.is_empty() && self.selected < self.entries.len() - 1 {
            self.selected += 1;
        }
    }

    /// Get the currently selected file path, if any.
    pub fn selected_path(&self) -> Option<&Path> {
        self.entries.get(self.selected).map(|p| p.as_path())
    }

    /// Get the list of entries.
    pub fn entries(&self) -> &[PathBuf] {
        &self.entries
    }

    /// Get the current selection index.
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// Get the current directory.
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Check if the file list is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Scan a directory for audio files with supported extensions.
fn scan_audio_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return files,
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                    files.push(path);
                }
            }
        }
    }

    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_file_browser_empty_dir() {
        let dir = std::env::temp_dir().join("tracker_rs_fb_empty");
        fs::create_dir_all(&dir).unwrap();

        let browser = FileBrowser::new(&dir);
        assert!(browser.is_empty());
        assert_eq!(browser.selected_index(), 0);
        assert!(browser.selected_path().is_none());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_file_browser_finds_audio_files() {
        let dir = std::env::temp_dir().join("tracker_rs_fb_audio");
        fs::create_dir_all(&dir).unwrap();

        // Create test files
        fs::write(dir.join("kick.wav"), b"fake").unwrap();
        fs::write(dir.join("snare.flac"), b"fake").unwrap();
        fs::write(dir.join("hihat.ogg"), b"fake").unwrap();
        fs::write(dir.join("readme.txt"), b"text").unwrap();
        fs::write(dir.join("image.png"), b"png").unwrap();

        let browser = FileBrowser::new(&dir);
        assert_eq!(browser.entries().len(), 3);

        // Should only contain audio files
        let names: Vec<&str> = browser
            .entries()
            .iter()
            .filter_map(|p| p.file_name().and_then(|n| n.to_str()))
            .collect();
        assert!(names.contains(&"kick.wav"));
        assert!(names.contains(&"snare.flac"));
        assert!(names.contains(&"hihat.ogg"));
        assert!(!names.contains(&"readme.txt"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_file_browser_navigation() {
        let dir = std::env::temp_dir().join("tracker_rs_fb_nav");
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("a.wav"), b"fake").unwrap();
        fs::write(dir.join("b.wav"), b"fake").unwrap();
        fs::write(dir.join("c.wav"), b"fake").unwrap();

        let mut browser = FileBrowser::new(&dir);
        assert_eq!(browser.selected_index(), 0);

        browser.move_down();
        assert_eq!(browser.selected_index(), 1);

        browser.move_down();
        assert_eq!(browser.selected_index(), 2);

        // Should not go past the end
        browser.move_down();
        assert_eq!(browser.selected_index(), 2);

        browser.move_up();
        assert_eq!(browser.selected_index(), 1);

        browser.move_up();
        assert_eq!(browser.selected_index(), 0);

        // Should not go before 0
        browser.move_up();
        assert_eq!(browser.selected_index(), 0);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_file_browser_selected_path() {
        let dir = std::env::temp_dir().join("tracker_rs_fb_sel");
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("sample.wav"), b"fake").unwrap();

        let browser = FileBrowser::new(&dir);
        let selected = browser.selected_path().unwrap();
        assert!(selected.file_name().unwrap().to_str().unwrap() == "sample.wav");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_file_browser_open_close() {
        let dir = std::env::temp_dir().join("tracker_rs_fb_oc");
        fs::create_dir_all(&dir).unwrap();

        let mut browser = FileBrowser::new(&dir);
        assert!(!browser.active);

        browser.open();
        assert!(browser.active);

        browser.close();
        assert!(!browser.active);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_file_browser_case_insensitive_extension() {
        let dir = std::env::temp_dir().join("tracker_rs_fb_case");
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("kick.WAV"), b"fake").unwrap();
        fs::write(dir.join("snare.Flac"), b"fake").unwrap();

        let browser = FileBrowser::new(&dir);
        assert_eq!(browser.entries().len(), 2);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_file_browser_nonexistent_dir() {
        let browser = FileBrowser::new(Path::new("/nonexistent/path/here"));
        assert!(browser.is_empty());
    }

    #[test]
    fn test_file_browser_sorted_entries() {
        let dir = std::env::temp_dir().join("tracker_rs_fb_sort");
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("zebra.wav"), b"fake").unwrap();
        fs::write(dir.join("alpha.wav"), b"fake").unwrap();
        fs::write(dir.join("mid.wav"), b"fake").unwrap();

        let browser = FileBrowser::new(&dir);
        let names: Vec<&str> = browser
            .entries()
            .iter()
            .filter_map(|p| p.file_name().and_then(|n| n.to_str()))
            .collect();
        assert_eq!(names, vec!["alpha.wav", "mid.wav", "zebra.wav"]);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_scan_audio_files() {
        let dir = std::env::temp_dir().join("tracker_rs_scan");
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("test.wav"), b"fake").unwrap();
        fs::write(dir.join("test.mp3"), b"fake").unwrap();

        let files = scan_audio_files(&dir);
        assert_eq!(files.len(), 1);
        assert!(files[0].file_name().unwrap().to_str().unwrap() == "test.wav");

        fs::remove_dir_all(&dir).ok();
    }
}
