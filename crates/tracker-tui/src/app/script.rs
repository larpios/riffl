use super::App;

impl App {
    /// Toggle live mode on/off.
    ///
    /// When live mode is active, scripts in the code editor are automatically
    /// re-evaluated on every pattern loop, allowing real-time algorithmic
    /// pattern generation during playback.
    pub fn toggle_live_mode(&mut self) {
        self.live_mode = !self.live_mode;
    }

    /// Execute the current script in the code editor.
    ///
    /// Scripts run in the main event loop (not the audio thread), so they never
    /// block audio rendering. When a script modifies the pattern during active
    /// playback, the mixer is retriggered on the current row so changes are
    /// immediately audible without waiting for the next row advance.
    pub fn execute_script(&mut self) {
        let code = self.code_editor.text();
        if code.trim().is_empty() {
            self.code_editor
                .set_output("(empty script)".to_string(), false);
            return;
        }

        match self
            .script_engine
            .eval_with_pattern(&code, self.editor.pattern())
        {
            Ok((result, commands)) => {
                // Apply pattern commands to the editor's pattern
                use tracker_core::dsl::engine::{apply_commands, ScriptResult};
                let cmd_count = commands.len();
                apply_commands(self.editor.pattern_mut(), &commands);

                // If playback is active and the script modified the pattern,
                // retrigger the mixer on the current row so changes are
                // immediately audible (not waiting for the next row advance).
                if cmd_count > 0 && self.transport.is_playing() {
                    if let Ok(mut mixer) = self.mixer.lock() {
                        mixer.tick(self.transport.current_row(), self.editor.pattern());
                    }
                }

                // Format output message
                let output_msg = if cmd_count > 0 {
                    match result {
                        ScriptResult::Value(v) => {
                            format!("Applied {} commands. Result: {}", cmd_count, v)
                        }
                        _ => format!("Applied {} commands to pattern.", cmd_count),
                    }
                } else {
                    match result {
                        ScriptResult::Value(v) => v,
                        ScriptResult::Unit => "(ok)".to_string(),
                        ScriptResult::PatternResult(_) => "(pattern result)".to_string(),
                    }
                };
                self.code_editor.set_output(output_msg, false);
            }
            Err(err) => {
                self.code_editor.set_output(err, true);
            }
        }
    }
}
