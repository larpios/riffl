use riffl_core::format::it::import_it;
use std::fs;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run -p riffl-core --example debug_it <file.it>");
        return;
    }

    let data = fs::read(&args[1]).expect("Failed to read file");
    let format_data = import_it(&data).expect("Failed to parse IT file");
    let song = &format_data.song;
    let samples = &format_data.samples;

    println!("=== Song: {} ===", song.name);
    println!(
        "BPM: {}, TPL: {}, Global Volume: {:.3}",
        song.bpm, song.tpl, song.global_volume
    );

    println!("=== Samples ({}) ===", samples.len());
    for (i, s) in samples.iter().enumerate() {
        println!(
            "  [{}] {:20} vol={:.3} default_vol_mult=?",
            i,
            s.name().unwrap_or("?"),
            s.volume,
        );
    }

    println!("=== Instruments ({}) ===", song.instruments.len());
    for (i, inst) in song.instruments.iter().enumerate() {
        println!(
            "  [{}] {:20} vol={:.4} sample={:?}",
            i, inst.name, inst.volume, inst.sample_index,
        );
    }
}
