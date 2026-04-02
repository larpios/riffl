use crate::app::App;
use crate::input::keybindings::Action;

pub(super) fn handle(app: &mut App, action: &Action) -> bool {
    match action {
        Action::SwitchView(view) => app.set_view(*view),
        Action::ToggleHelp => {
            app.show_help = !app.show_help;
            app.help_scroll = 0;
        }
        Action::ToggleEffectHelp => {
            app.show_effect_help = !app.show_effect_help;
            app.effect_help_scroll = 0;
        }
        Action::ShowWhichKey => app.which_key_mode = !app.which_key_mode,
        Action::OpenFileBrowser => app.open_file_browser(),
        Action::OpenModuleBrowser => app.open_module_browser(),
        Action::ToggleSplitView => app.toggle_split_view(),
        Action::ToggleLiveMode => app.toggle_live_mode(),
        Action::ToggleFollowMode => app.follow_mode = !app.follow_mode,
        Action::ToggleDrawMode => app.toggle_draw_mode(),
        Action::ExecuteScript => app.execute_script(&[]),
        Action::ExecuteScriptOnSelection => app.execute_script_on_selection(),
        Action::OpenTemplates => app.code_editor.toggle_templates(),
        _ => return false,
    }
    true
}
