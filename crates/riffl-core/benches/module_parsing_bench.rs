use criterion::{black_box, criterion_group, criterion_main, Criterion};
use riffl_core::format;

fn bench_parse_s3m(c: &mut Criterion) {
    let data = std::fs::read("/Users/ray/.config/riffl/samples/2nd_pm.s3m")
        .expect("Failed to read S3M file");

    c.bench_function("parse_s3m", |b| {
        b.iter(|| {
            let result = format::load(black_box(&data));
            let _ = black_box(result);
        });
    });
}

fn bench_parse_it(c: &mut Criterion) {
    let data = std::fs::read("/Users/ray/.config/riffl/samples/D_BIND.it")
        .expect("Failed to read IT file");

    c.bench_function("parse_it", |b| {
        b.iter(|| {
            let result = format::load(black_box(&data));
            let _ = black_box(result);
        });
    });
}

fn bench_parse_mod(c: &mut Criterion) {
    let data = std::fs::read("/Users/ray/.config/riffl/samples/0u7r4g30us_v1b35.mod")
        .expect("Failed to read MOD file");

    c.bench_function("parse_mod", |b| {
        b.iter(|| {
            let result = format::load(black_box(&data));
            let _ = black_box(result);
        });
    });
}

fn bench_parse_xm(c: &mut Criterion) {
    let data = std::fs::read("/Users/ray/.config/riffl/samples/BUTTERFL.XM")
        .expect("Failed to read XM file");

    c.bench_function("parse_xm", |b| {
        b.iter(|| {
            let result = format::load(black_box(&data));
            let _ = black_box(result);
        });
    });
}

fn bench_parse_s3m_detect(c: &mut Criterion) {
    let data = std::fs::read("/Users/ray/.config/riffl/samples/2nd_pm.s3m")
        .expect("Failed to read S3M file");

    c.bench_function("parse_s3m_with_detect", |b| {
        b.iter(|| {
            let result = format::load(black_box(&data));
            let _ = black_box(result);
        });
    });
}

criterion_group!(
    benches,
    bench_parse_s3m,
    bench_parse_it,
    bench_parse_mod,
    bench_parse_xm,
    bench_parse_s3m_detect
);
criterion_main!(benches);
