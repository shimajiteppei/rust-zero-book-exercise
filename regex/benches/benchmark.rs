use criterion::{criterion_group, criterion_main, Criterion};
use regex::engine::do_matching;
use std::time::Duration;

const REDOS_REGEX: &[(&str, &str, &str)] = &[
    ("Cox:n=02", "a?a?aa", "aa"),
    ("Cox:n=04", "a?a?a?a?aaaa", "aaaa"),
    ("Cox:n=08", "a?a?a?a?a?a?a?a?aaaaaaaa", "aaaaaaaa"),
    ("Cox:n=16", "a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?aaaaaaaaaaaaaaaa","aaaaaaaaaaaaaaaa"),
    ("Cox:n=32", "a?a?a?a?a?a?a?a??a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    ("Cox:n=64", "a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    ("Cox:n=128", "a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?a?aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    ("nested plus:n=02", "(a+)+", "aa"),
    ("nested plus:n=04", "(a+)+", "aaaa"),
    ("nested plus:n=08", "(a+)+", "aaaaaaaa"),
    ("nested plus:n=16", "(a+)+", "aaaaaaaaaaaaaaaa"),
    ("nested plus:n=32", "(a+)+", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    ("nested plus:n=64", "(a+)+", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    ("nested plus:n=128", "(a+)+", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    ("Cox like:n=02", "(a|a?)+", "aa"),
    ("Cox like:n=04", "(a|a?)+", "aaaa"),
    ("Cox like:n=08", "(a|a?)+", "aaaaaaaa"),
    ("Cox like:n=16", "(a|a?)+", "aaaaaaaaaaaaaaaa"),
    ("Cox like:n=32", "(a|a?)+", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    ("Cox like:n=64", "(a|a?)+", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    ("Cox like:n=128", "(a|a?)+", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
];

fn width_first(c: &mut Criterion) {
    let mut g = c.benchmark_group("Width First");
    g.measurement_time(Duration::from_secs(5));

    for i in REDOS_REGEX {
        g.bench_with_input(i.0, &(i.1, i.2), |b, args| {
            b.iter(|| do_matching(args.0, args.1, false))
        });
    }
}

fn depth_first(c: &mut Criterion) {
    let mut g = c.benchmark_group("Depth First");
    g.measurement_time(Duration::from_secs(5));

    for i in REDOS_REGEX {
        g.bench_with_input(i.0, &(i.1, i.2), |b, args| {
            b.iter(|| do_matching(args.0, args.1, true))
        });
    }
}

criterion_group!(benches, width_first);
// criterion_group!(benches, width_first, depth_first);
criterion_main!(benches);
