#![feature(custom_test_frameworks)]
#![test_runner(criterion::runner)]
use criterion::{Criterion, black_box};
use criterion_macro::criterion;
use log::info;
use datex_core::runtime::Runtime;

// simple runtime initialization
fn runtime_init() {
    let runtime = Runtime::default();
    info!("Runtime version: {}", runtime.version);
}

#[criterion]
fn bench_simple(c: &mut Criterion) {
    c.bench_function("runtime init", |b| b.iter(|| runtime_init()));
}