use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use yfix::config::Config;
use yfix::processor::Processor;

static FIXTURES: &[(&str, &str)] = &[
    ("small", include_str!("fixtures/small.txt")),
    ("medium", include_str!("fixtures/medium.txt")),
    ("large", include_str!("fixtures/large.txt")),
];

fn processor_process(c: &mut Criterion) {
    let config = Config::default();
    let processor = Processor::from_config(&config, 80);

    let mut group = c.benchmark_group("processor_process");
    for &(size, input) in FIXTURES {
        group.bench_with_input(BenchmarkId::from_parameter(size), input, |b, text| {
            b.iter(|| processor.process(text).unwrap());
        });
    }
    group.finish();
}

fn config_load(c: &mut Criterion) {
    c.bench_function("config_load_default", |b| {
        b.iter(|| Config::load(None).unwrap());
    });
}

criterion_group!(benches, processor_process, config_load);
criterion_main!(benches);
