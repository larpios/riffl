#[test]
fn test_load_real_s3m_files() {
    let paths = vec![
        "/Users/ray/.config/riffl/samples/2nd_pm.s3m",
        "/Users/ray/.config/riffl/samples/DISTANCE.S3M",
        "/Users/ray/.config/riffl/samples/pod.s3m",
        "/Users/ray/.config/riffl/samples/celestial_fantasia.s3m",
        "/Users/ray/.config/riffl/samples/aryx.s3m",
        "/Users/ray/.config/riffl/samples/skyrider.s3m",
    ];

    for path in paths {
        eprintln!("\n=== Testing: {} ===", path);
        let data = std::fs::read(path).expect("Failed to read file");
        eprintln!("File size: {} bytes", data.len());

        match tracker_core::format::s3m::import_s3m(&data) {
            Ok(format_data) => {
                eprintln!("SUCCESS: song='{}'", format_data.song.name);
                eprintln!("  Instruments: {}", format_data.song.instruments.len());
                eprintln!("  Samples: {}", format_data.samples.len());

                // Verify samples have actual audio data (not sine wave fallback)
                let mut samples_with_data = 0;
                let mut empty_samples = 0;
                for sample in format_data.samples.iter() {
                    let data_len = sample.data().len();
                    if data_len > 0 {
                        samples_with_data += 1;
                    } else {
                        empty_samples += 1;
                    }
                }
                eprintln!("  Samples with audio data: {}", samples_with_data);
                eprintln!("  Empty samples: {}", empty_samples);

                // Verify at least some samples have actual audio data
                assert!(samples_with_data > 0, "No samples with audio data found");
            }
            Err(e) => {
                eprintln!("ERROR: {:?}", e);
                panic!("Failed to load {}", path);
            }
        }
    }
}
