mod audio;

use audio::AudioEngine;
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn main() {
    println!("Audio Engine - Test Tone Demo");
    println!("==============================");
    println!();

    // Create the audio engine with default device and configuration
    let mut engine = match AudioEngine::new() {
        Ok(e) => {
            println!("✓ AudioEngine initialized successfully");
            println!("  Sample rate: {}Hz", e.sample_rate());
            println!("  Device: {}", e.device().name().unwrap_or_else(|_| "Unknown".to_string()));
            println!("  Latency: {:.2}ms (theoretical)", e.latency_ms());
            e
        }
        Err(e) => {
            eprintln!("✗ Failed to initialize AudioEngine: {:?}", e);
            eprintln!("  Make sure you have an audio output device available.");
            return;
        }
    };

    println!();
    println!("Generating 440Hz sine wave (A4 note)...");
    println!();

    // Create a callback that generates a 440Hz sine wave
    let frequency = 440.0; // A4 note
    let sample_rate = engine.sample_rate() as f32;
    let amplitude = 0.3; // Reduced amplitude to avoid clipping

    // Phase accumulator for continuous sine wave generation
    // We need to track phase across callback invocations to avoid discontinuities
    let phase = Arc::new(Mutex::new(0.0f32));
    let phase_clone = phase.clone();

    let callback = Arc::new(Mutex::new(move |data: &mut [f32]| {
        let mut current_phase = phase_clone.lock().unwrap();

        // Calculate phase increment per sample
        let phase_increment = 2.0 * PI * frequency / sample_rate;

        // Generate sine wave samples
        for sample in data.iter_mut() {
            *sample = amplitude * (*current_phase).sin();
            *current_phase += phase_increment;

            // Keep phase in range [0, 2*PI) to avoid floating point precision issues
            if *current_phase >= 2.0 * PI {
                *current_phase -= 2.0 * PI;
            }
        }
    }));

    // Register the callback with the engine
    match engine.set_callback(callback) {
        Ok(_) => println!("✓ Audio callback registered"),
        Err(e) => {
            eprintln!("✗ Failed to set callback: {:?}", e);
            return;
        }
    }

    // Start playback
    match engine.start() {
        Ok(_) => {
            println!("✓ Playback started");
            println!();
            println!("Playing 440Hz test tone for 5 seconds...");
            println!("(You should hear a continuous tone)");
        }
        Err(e) => {
            eprintln!("✗ Failed to start playback: {:?}", e);
            return;
        }
    }

    // Play for 5 seconds
    std::thread::sleep(Duration::from_secs(5));

    // Stop playback
    println!();
    println!("Stopping playback...");
    engine.stop();

    // Give the audio system time to cleanly shutdown
    std::thread::sleep(Duration::from_millis(100));

    println!("✓ Playback stopped cleanly");
    println!();
    println!("Demo complete!");
}
