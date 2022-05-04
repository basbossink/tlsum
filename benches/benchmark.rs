use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::env;
use std::path::PathBuf;

use lib::{now, summarize_file};

pub fn summarize_lines_bench(c: &mut Criterion) {
    let mut path = PathBuf::new();
    path.push(env::var("CARGO_MANIFEST_DIR").unwrap());
    path.push("benches");
    path.set_file_name("anonimized-timelog");
    path.set_extension("txt");
    let now = now().unwrap();

    c.bench_with_input(
        BenchmarkId::new("summarize_lines", "anonimized-timelog"),
        &(&path, &now),
        |b, s| {
            b.iter(|| {
                let (path, now) = s;
                summarize_file(path, *now)
            })
        },
    );
}

criterion_group!(benches, summarize_lines_bench);
criterion_main!(benches);
