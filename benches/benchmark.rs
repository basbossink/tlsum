use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::env;
use std::path::PathBuf;

use lib::summarize_lines;

pub fn summarize_lines_bench(c: &mut Criterion) {
    let mut path = PathBuf::new();
    path.push(env::var("CARGO_MANIFEST_DIR").unwrap());
    path.push("benches");
    path.set_file_name("anonimized-timelog");
    path.set_extension("txt");

    c.bench_with_input(
        BenchmarkId::new("summarize_lines", "anonimized-timelog"),
        &path,
        |b, s| b.iter(|| summarize_lines(s)),
    );
}

criterion_group!(benches, summarize_lines_bench);
criterion_main!(benches);
