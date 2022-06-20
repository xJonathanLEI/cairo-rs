use cleopatra_cairo::cairo_run;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("cairo_run(bench/criterion/fibonacci_1000.json", |b| {
        b.iter(|| cairo_run::cairo_run(black_box("bench/criterion/fibonacci_1000.json")))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);