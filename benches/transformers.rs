use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use yfix::transformer::{
    Transformer, compress_blank::CompressBlank, dedent::Dedent, join_wrapped::JoinWrapped,
    strip_ansi::StripAnsi, strip_line_numbers::StripLineNumbers, strip_prompt::StripPrompt,
    strip_trailing::StripTrailing,
};

static FIXTURES: &[(&str, &str)] = &[
    ("small", include_str!("fixtures/small.txt")),
    ("medium", include_str!("fixtures/medium.txt")),
    ("large", include_str!("fixtures/large.txt")),
];

fn bench_transformer(c: &mut Criterion, name: &str, t: &dyn Transformer) {
    let mut group = c.benchmark_group(name);
    for &(size, input) in FIXTURES {
        group.bench_with_input(BenchmarkId::from_parameter(size), input, |b, text| {
            b.iter(|| t.transform(text).unwrap());
        });
    }
    group.finish();
}

fn strip_ansi(c: &mut Criterion) {
    bench_transformer(c, "strip_ansi", &StripAnsi);
}

fn strip_line_numbers(c: &mut Criterion) {
    bench_transformer(c, "strip_line_numbers", &StripLineNumbers);
}

fn join_wrapped(c: &mut Criterion) {
    bench_transformer(
        c,
        "join_wrapped",
        &JoinWrapped {
            wrap_width: 80,
            skip_table_lines: true,
        },
    );
}

fn dedent(c: &mut Criterion) {
    bench_transformer(c, "dedent", &Dedent);
}

fn strip_trailing(c: &mut Criterion) {
    bench_transformer(c, "strip_trailing", &StripTrailing);
}

fn compress_blank(c: &mut Criterion) {
    bench_transformer(c, "compress_blank", &CompressBlank);
}

fn strip_prompt(c: &mut Criterion) {
    bench_transformer(c, "strip_prompt", &StripPrompt);
}

criterion_group!(
    benches,
    strip_ansi,
    strip_line_numbers,
    join_wrapped,
    dedent,
    strip_trailing,
    compress_blank,
    strip_prompt,
);
criterion_main!(benches);
