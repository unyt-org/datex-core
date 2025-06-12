#![feature(custom_test_frameworks)]
#![test_runner(criterion::runner)]
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use criterion_macro::criterion;
use datex_core::compiler::bytecode::compile_script;
use crate::json::{get_json_test_string, json_to_runtime_value_baseline_serde};
use crate::runtime::runtime_init;

mod runtime;
mod json;


fn bench_runtime(c: &mut Criterion) {
    c.bench_function("runtime init", |b| b.iter(|| runtime_init()));
}

fn bench_json(c: &mut Criterion) {
    // c.bench_function("runtime init", |b| b.iter(|| runtime_init()));

    // JSON benchmarks
    // JSON string to runtime value
    let json = get_json_test_string();
    let dxb  = compile_script(&json).expect("Failed to parse JSON string");
    // serde
    c.bench_function("json to runtime value", |b| {
        b.iter(|| {
            black_box(json_to_runtime_value_baseline_serde(black_box(&json)));
        })
    });
    // json_syntax
    c.bench_function("json_syntax to runtime value", |b| {
        b.iter(|| {
            black_box(json::json_to_runtime_value_baseline_json_syntax(black_box(&json)));
        })
    });
    // DATEX
    c.bench_function("datex to runtime value", |b| {
        b.iter(|| {
            black_box(json::json_to_runtime_value_datex(black_box(&json)));
        })
    });
    // DXB
    c.bench_function("dxb to runtime value", |b| {
        b.iter(|| {
            black_box(json::dxb_to_runtime_value(black_box(&dxb)));
        })
    });

    // runtime value to JSON
    let json_serde = json::json_to_serde_value(&json);
    let json_syntax = json::json_to_json_syntax_value(&json);
    let json_datex = json::json_to_datex_value(&json);
    // serde
    c.bench_function("runtime value to serde JSON", |b| {
        b.iter(|| {
            black_box(json::runtime_value_to_json_baseline_serde_json(black_box(&json_serde)));
        })
    });
    // json_syntax
    c.bench_function("runtime value to json_syntax JSON", |b| {
        b.iter(|| {
            black_box(json::runtime_value_to_json_baseline_json_syntax(black_box(&json_syntax)));
        })
    });
    // DATEX
    c.bench_function("runtime value to datex JSON", |b| {
        b.iter(|| {
            black_box(json::runtime_value_to_json_datex(black_box(&json_datex)));
        })
    });
    // DXB
    c.bench_function("runtime value to DXB", |b| {
        b.iter(|| {
            black_box(json::runtime_value_to_dxb(black_box(&json_datex)));
        })
    });
    // DXB to JSON
    c.bench_function("DXB to JSON", |b| {
        b.iter(|| {
            black_box(json::dxb_to_json(black_box(&dxb)));
        })
    });
}

criterion_group!(json, bench_json);
criterion_group!(runtime, bench_runtime);

criterion_main!(json);
