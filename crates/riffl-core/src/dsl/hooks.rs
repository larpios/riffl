/// User-scriptable hook system.
///
/// Place `~/.config/riffl/hooks.rhai` to customise behaviour. Any subset of
/// the supported hook functions can be defined; undefined hooks fall back to
/// sensible built-in defaults.
///
/// # Supported hooks
///
/// ```rhai
/// // Transform raw picker output to a plain file path.
/// // Called before riffl resolves relative paths or opens the file.
/// // Returning the value unchanged lets the built-in normalisation run.
/// fn normalize_picker_path(raw) { raw }
///
/// // Called after a project (.rtm) file is successfully loaded.
/// fn on_project_loaded(path) {}
///
/// // Called after a sample is loaded into instrument slot `inst_idx`.
/// fn on_sample_loaded(path, inst_idx) {}
///
/// // Called once when riffl starts, after config is loaded.
/// fn on_startup() {}
/// ```
use rhai::{Dynamic, Engine, Scope, AST};
use std::path::Path;

pub struct HooksEngine {
    engine: Engine,
    ast: Option<AST>,
}

impl HooksEngine {
    /// Load hooks from `path`. If the file does not exist the engine is still
    /// usable — all calls fall back to built-in defaults silently.
    pub fn load(path: &Path) -> Self {
        let engine = Engine::new();
        let ast = if path.exists() {
            match std::fs::read_to_string(path) {
                Ok(src) => match engine.compile(&src) {
                    Ok(ast) => Some(ast),
                    Err(e) => {
                        eprintln!("riffl: hooks.rhai compile error: {e}");
                        None
                    }
                },
                Err(e) => {
                    eprintln!("riffl: could not read hooks.rhai: {e}");
                    None
                }
            }
        } else {
            None
        };
        Self { engine, ast }
    }

    /// Transform raw picker output to a clean path string.
    ///
    /// If `normalize_picker_path` is defined in hooks.rhai its return value is
    /// used directly. Otherwise the built-in default strips yazi's
    /// `search://<query>//<path>` URI format.
    pub fn normalize_picker_path(&self, raw: &str) -> String {
        if let Some(result) = self.call_str("normalize_picker_path", (raw.to_string(),)) {
            return result;
        }
        // Built-in default: strip yazi search-mode URI prefix
        // Format: search://<query>:<line>:<col>//<absolute_path>
        if let Some(rest) = raw.strip_prefix("search://") {
            if let Some(pos) = rest.find("//") {
                return rest[pos + 1..].to_string();
            }
        }
        raw.to_string()
    }

    /// Called after a project file is successfully loaded.
    pub fn on_project_loaded(&self, path: &str) {
        self.call_void("on_project_loaded", (path.to_string(),));
    }

    /// Called after a sample is loaded into instrument slot `inst_idx`.
    pub fn on_sample_loaded(&self, path: &str, inst_idx: i64) {
        self.call_void("on_sample_loaded", (path.to_string(), inst_idx));
    }

    /// Called once on application startup.
    pub fn on_startup(&self) {
        self.call_void("on_startup", ());
    }

    // --- helpers ---

    fn call_str(&self, name: &str, args: impl rhai::FuncArgs) -> Option<String> {
        let ast = self.ast.as_ref()?;
        let mut scope = Scope::new();
        match self.engine.call_fn::<String>(&mut scope, ast, name, args) {
            Ok(v) => Some(v),
            Err(e) => self.handle_call_err(name, &e),
        }
    }

    fn call_void(&self, name: &str, args: impl rhai::FuncArgs) {
        let Some(ast) = self.ast.as_ref() else {
            return;
        };
        let mut scope = Scope::new();
        match self.engine.call_fn::<Dynamic>(&mut scope, ast, name, args) {
            Ok(_) => {}
            Err(e) => {
                self.handle_call_err::<()>(name, &e);
            }
        }
    }

    /// Returns `None` for "function not found" (not an error), logs and returns
    /// `None` for any other error.
    fn handle_call_err<T>(&self, name: &str, e: &rhai::EvalAltResult) -> Option<T> {
        if matches!(e, rhai::EvalAltResult::ErrorFunctionNotFound(..)) {
            return None;
        }
        eprintln!("riffl: hooks.rhai {name}(): {e}");
        None
    }
}
