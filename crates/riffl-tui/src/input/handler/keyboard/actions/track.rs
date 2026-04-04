use crate::app::App;
use crate::input::keybindings::Action;

pub(super) fn handle(app: &mut App, action: &Action) -> bool {
    match action {
        Action::AddTrack => {
            app.editor.add_track();
            app.sync_mixer_channels();
            app.mark_dirty();
        }
        Action::DeleteTrack => {
            app.editor.delete_track();
            app.sync_mixer_channels();
            app.mark_dirty();
        }
        Action::CloneTrack => {
            app.editor.clone_track();
            app.sync_mixer_channels();
            app.mark_dirty();
        }
        Action::ToggleMute => app.toggle_mute_current_track(),
        Action::ToggleSolo => app.toggle_solo_current_track(),
        Action::TrackVolumeUp => app.adjust_track_volume(0.05),
        Action::TrackVolumeDown => app.adjust_track_volume(-0.05),
        Action::TrackPanLeft => app.adjust_track_pan(-0.1),
        Action::TrackPanRight => app.adjust_track_pan(0.1),
        Action::NextTrack => app.editor.next_track(),
        Action::PrevTrack => app.editor.prev_track(),
        _ => return false,
    }
    true
}
