#[test]
fn debug_2nd_pm_s3m() {
    use std::fs;
    use tracker_core::format::s3m::*;

    let test_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("test_modules");
    let path = test_dir.join("2nd_pm.s3m");

    println!("=== Debug parsing 2nd_pm.s3m ===");

    let data = fs::read(&path).expect("Failed to read file");
    println!("File size: {} bytes", data.len());

    // Parse header manually first
    let header = parse_s3m_header(&data).expect("Failed to parse header");
    let signed_samples = header.ffi == 1;
    println!("Header:");
    println!("  name: '{}'", header.name);
    println!("  ord_num: {}", header.ord_num);
    println!("  ins_num: {}", header.ins_num);
    println!("  pat_num: {}", header.pat_num);
    println!("  ffi: {}", header.ffi);
    println!("  initial_speed: {}", header.initial_speed);
    println!("  initial_tempo: {}", header.initial_tempo);
    println!("  global_vol: {}", header.global_vol);

    println!("Instrument pointers: {:?}", header.inst_pointers);
    println!("Pattern pointers: {:?}", header.pat_pointers);

    // Parse each instrument
    for (i, &ptr) in header.inst_pointers.iter().enumerate() {
        if ptr == 0 {
            continue;
        }
        println!("\n--- Instrument {} (para_ptr={:04x}) ---", i, ptr);
        match parse_s3m_instrument(&data, ptr, signed_samples) {
            Ok(inst) => {
                println!("  name: '{}'", inst.name);
                println!(
                    "  type: {}",
                    if inst.sample_data.is_some() {
                        "PCM"
                    } else {
                        "Adlib/Empty"
                    }
                );
                if let Some(ref sample_data) = inst.sample_data {
                    println!("  sample_len: {} samples", sample_data.len());
                    let data_len = sample_data.len();
                    println!("  sample_data len: {} frames", data_len);
                    println!("  loop_begin: {}", inst.loop_begin);
                    println!("  loop_end: {}", inst.loop_end);
                    println!("  volume: {}", inst.volume);
                    println!("  flags: {:08b}", inst.flags);
                    println!("  c2spd: {}", inst.c2spd);

                    // Check if sample data looks valid
                    let mut min_val = f32::MAX;
                    let mut max_val = f32::MIN;
                    for &v in sample_data.iter().take(100) {
                        min_val = min_val.min(v);
                        max_val = max_val.max(v);
                    }
                    println!("  sample range (first 100): {} to {}", min_val, max_val);

                    // Check flags
                    let is_16bit = inst.flags & 4 != 0;
                    let is_looped = inst.flags & 1 != 0;
                    let is_stereo = inst.flags & 2 != 0;
                    println!(
                        "  is_16bit: {}, is_looped: {}, is_stereo: {}",
                        is_16bit, is_looped, is_stereo
                    );
                } else {
                    println!("  No sample data");
                }
            }
            Err(e) => {
                println!("  ERROR: {}", e);
            }
        }
    }

    // Now try full import
    println!("\n=== Full import ===");
    match import_s3m(&data) {
        Ok(format_data) => {
            let song = &format_data.song;
            let samples = &format_data.samples;

            println!("Song: '{}'", song.name);
            println!("Instruments: {}", song.instruments.len());
            for (i, inst) in song.instruments.iter().enumerate() {
                println!(
                    "  [{}] '{}' sample_index: {:?}",
                    i, inst.name, inst.sample_index
                );
            }

            println!("Samples: {}", samples.len());
            for (i, sample) in samples.iter().enumerate() {
                println!("  [{}] name: '{}'", i, sample.name().unwrap_or("?"));
                println!("     data len: {} frames", sample.data().len());
                println!("     sample rate: {}", sample.sample_rate());
                println!("     volume: {}", sample.volume);
                println!("     base_note: {}", sample.base_note());
                println!("     loop_mode: {:?}", sample.loop_mode);
                if sample.loop_mode != tracker_core::audio::LoopMode::NoLoop {
                    println!(
                        "     loop_start: {}, loop_end: {}",
                        sample.loop_start, sample.loop_end
                    );
                }
            }

            println!("Patterns: {}", song.patterns.len());
            println!("Arrangement: {:?}", song.arrangement);
            println!("Tracks: {}", song.tracks.len());
        }
        Err(e) => {
            println!("ERROR: {}", e);
        }
    }
}
