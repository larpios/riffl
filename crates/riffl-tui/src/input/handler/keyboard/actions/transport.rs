use crate::app::{App, AppView};
use crate::input::keybindings::Action;

pub(super) fn handle(app: &mut App, action: &Action) -> bool {
    match action {
        Action::TogglePlay => {
            if app.current_view == AppView::InstrumentList {
                if let Some(idx) = app.instrument_selection() {
                    use riffl_core::pattern::note::Pitch;
                    app.preview_instrument_note_pitch(idx, Pitch::C, 4);
                }
            } else {
                app.toggle_play();
            }
        }
        Action::PlayFromCursor => app.play_from_cursor(),
        Action::Stop => app.stop(),
        Action::BpmUp => app.adjust_bpm(1.0),
        Action::BpmDown => app.adjust_bpm(-1.0),
        Action::BpmUpLarge => app.adjust_bpm(10.0),
        Action::BpmDownLarge => app.adjust_bpm(-10.0),
        Action::ToggleLoop => app.toggle_loop(),
        Action::ToggleMetronome => app.toggle_metronome(),
        Action::TogglePlaybackMode => app.toggle_playback_mode(),
        Action::JumpNextPattern => app.jump_next_pattern(),
        Action::JumpPrevPattern => app.jump_prev_pattern(),
        Action::OpenBpmPrompt => app.open_bpm_prompt(),
        Action::TapTempo => app.tap_tempo(),
        Action::OpenLenPrompt => app.open_len_prompt(),
        Action::SetLoopStart => app.set_loop_start(),
        Action::SetLoopEnd => app.set_loop_end(),
        Action::ToggleLoopRegion => app.toggle_loop_region_active(),
        _ => return false,
    }
    true
}
