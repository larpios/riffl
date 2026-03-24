#[test]
fn test_load_real_s3m_files() {
    let paths = vec![
        "/Users/ray/.config/riffl/samples/2nd_pm.s3m",
        "/Users/ray/.config/riffl/samples/DISTANCE.S3M",
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
            }
            Err(e) => {
                eprintln!("ERROR: {:?}", e);
                panic!("Failed to load {}", path);
            }
        }
    }
}
