use tracker_core::format::{it::import_it, protracker::import_mod, xm::import_xm};

fn test_modules_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("test_modules")
}

fn load_files(extensions: &[&str]) -> Vec<std::path::PathBuf> {
    let dir = test_modules_dir();
    std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| extensions.iter().any(|&x| ext.eq_ignore_ascii_case(x)))
                        .unwrap_or(false)
                })
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default()
}

fn import_xm_file(path: &std::path::Path) -> Result<tracker_core::format::FormatData, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    import_xm(&data).map_err(|e| format!("{:?}", e))
}

fn import_mod_file(path: &std::path::Path) -> Result<tracker_core::format::FormatData, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    import_mod(&data).map_err(|e| format!("{:?}", e))
}

fn import_it_file(path: &std::path::Path) -> Result<tracker_core::format::FormatData, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    import_it(&data).map_err(|e| format!("{:?}", e))
}

#[test]
fn test_load_xm_files() {
    let paths = load_files(&["xm"]);
    if paths.is_empty() {
        eprintln!("No XM files found, skipping test");
        return;
    }

    let mut loaded = 0;
    let mut failed = 0;

    for path in &paths {
        match import_xm_file(path) {
            Ok(format_data) => {
                let has_samples = format_data.samples.iter().any(|s| !s.data().is_empty());
                if has_samples {
                    loaded += 1;
                }
            }
            Err(e) => {
                eprintln!("FAILED: {:?} — {}", path, e);
                failed += 1;
            }
        }
    }

    eprintln!("XM: {}/{} loaded ({} failed)", loaded, paths.len(), failed);
    assert!(loaded > 0, "No XM files loaded successfully");
}

#[test]
fn test_load_it_files() {
    let paths = load_files(&["it"]);
    if paths.is_empty() {
        eprintln!("No IT files found, skipping test");
        return;
    }

    let mut loaded = 0;
    let mut failed = 0;

    for path in &paths {
        match import_it_file(path) {
            Ok(format_data) => {
                let has_samples = format_data.samples.iter().any(|s| !s.data().is_empty());
                if has_samples {
                    loaded += 1;
                }
            }
            Err(e) => {
                eprintln!("FAILED: {:?} — {}", path, e);
                failed += 1;
            }
        }
    }

    eprintln!("IT: {}/{} loaded ({} failed)", loaded, paths.len(), failed);
    assert!(loaded > 0, "No IT files loaded successfully");
}

#[test]
fn test_load_mod_files() {
    let paths = load_files(&["mod"]);
    if paths.is_empty() {
        eprintln!("No MOD files found, skipping test");
        return;
    }

    let mut loaded = 0;
    let mut failed = 0;

    for path in &paths {
        match import_mod_file(path) {
            Ok(format_data) => {
                let has_samples = format_data.samples.iter().any(|s| !s.data().is_empty());
                if has_samples {
                    loaded += 1;
                }
            }
            Err(e) => {
                eprintln!("FAILED: {:?} — {}", path, e);
                failed += 1;
            }
        }
    }

    eprintln!("MOD: {}/{} loaded ({} failed)", loaded, paths.len(), failed);
    assert!(loaded > 0, "No MOD files loaded successfully");
}
