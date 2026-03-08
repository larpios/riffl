use criterion::{black_box, criterion_group, criterion_main, Criterion};
use riffl_core::audio::mixer::Mixer;
use riffl_core::audio::sample::Sample;

fn bench_mixer_new(c: &mut Criterion) {
    // Large sample data to simulate real-world usage (10 seconds @ 44.1kHz)
    let sample_data = vec![0.0f32; 44100 * 10];
    let sample = Sample::new(sample_data, 44100, 1, None);
    let sample_arc = std::sync::Arc::new(sample);

    c.bench_function("Mixer::new with Arc<Sample>", |b| {
        b.iter(|| {
            let mixer = Mixer::new(
                black_box(vec![std::sync::Arc::clone(&sample_arc)]),
                Vec::new(),
                4,
                44100,
            );
            black_box(mixer);
        });
    });
}

criterion_group!(benches, bench_mixer_new);
criterion_main!(benches);
