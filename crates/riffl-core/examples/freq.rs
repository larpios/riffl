fn main() {
    let note = 48.0; // C-4
    let finetune = 0.0;

    // FT2 Amiga formula roughly
    let period = 7680.0 - note * 64.0 - finetune / 2.0;
    let freq = 8363.0 * 2.0_f64.powf((4608.0 - period) as f64 / 768.0);
    println!("FT2 C-4 frequency is {} Hz", freq);

    let note_5 = 60.0; // C-5
    let period_5 = 7680.0 - note_5 * 64.0 - finetune / 2.0;
    let freq_5 = 8363.0 * 2.0_f64.powf((4608.0 - period_5) as f64 / 768.0);
    println!("FT2 C-5 frequency is {} Hz", freq_5);

    // Test my riffl-core math
    let base_note = 48; // relative pitch 0
    let play_note = 48;

    // frequency calculation in my engine (A4 = 57, A4 = 440)
    let a4_midi = 57;
    let base_freq = 440.0 * 2.0_f64.powf((base_note - a4_midi) as f64 / 12.0);
    let target_freq = 440.0 * 2.0_f64.powf((play_note - a4_midi) as f64 / 12.0);
    let ratio = target_freq / base_freq;
    println!("Tracker Core C-4 play ratio: {}", ratio);
    println!(
        "Tracker Core native rate for C-4: {} * {} = {}",
        8363.0,
        ratio,
        8363.0 * ratio
    );

    // IF my tracker parses C-4 as MIDI 60:
    let play_note_wrong = 60;
    let target_freq_wrong = 440.0 * 2.0_f64.powf((play_note_wrong - a4_midi) as f64 / 12.0);
    println!(
        "Tracker Core C-5 play ratio: {}",
        target_freq_wrong / base_freq
    );

    // ProTracker C-2 is MIDI 36.
    let base_note_pt = 36;
    let play_note_pt = 36;
    let base_freq_pt = 440.0 * 2.0_f64.powf((base_note_pt - a4_midi) as f64 / 12.0);
    let target_freq_pt = 440.0 * 2.0_f64.powf((play_note_pt - a4_midi) as f64 / 12.0);
    println!(
        "Tracker Core PT C-2 play ratio: {}",
        target_freq_pt / base_freq_pt
    );
}
