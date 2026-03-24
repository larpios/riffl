use tracker_core::format::protracker::ModLoader;
use tracker_core::format::ModuleLoader;

#[test]
fn test_detect_st15() {
    let mut data = vec![0u8; 600];
    // Title
    data[0..4].copy_from_slice(b"test");
    // Song length at 470
    data[470] = 1;
    // Pattern order table at 472
    data[472] = 0;
    
    let loader = ModLoader;
    assert!(loader.detect(&data));
}

#[test]
fn test_load_st15_minimal() {
    let mut data = vec![0u8; 600 + 1024]; // 600 header + 1 pattern (1024 bytes)
    data[470] = 1; // length
    data[472] = 0; // pattern 0
    
    let loader = ModLoader;
    let result = loader.load(&data);
    assert!(result.is_ok(), "ST15 load failed: {:?}", result.err());
    let format_data = result.unwrap();
    assert_eq!(format_data.song.instruments.len(), 15);
    assert_eq!(format_data.samples.len(), 15);
    assert_eq!(format_data.song.patterns.len(), 1);
    assert_eq!(format_data.song.patterns[0].num_channels(), 4);
}
