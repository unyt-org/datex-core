#![feature(custom_test_frameworks)]
#![test_runner(criterion::runner)]
use criterion::{Criterion, black_box, criterion_group, criterion_main, BenchmarkId};
use criterion_macro::criterion;
use datex_core::compiler::bytecode::compile_script;
use crate::json::{get_json_test_string, json_to_dxb, json_to_runtime_value_baseline_serde};
use crate::runtime::runtime_init;

mod runtime;
mod json;


fn bench_runtime(c: &mut Criterion) {
    c.bench_function("runtime init", |b| b.iter(|| runtime_init()));
}

fn bench_json_file(c: &mut Criterion, file_path: &str) {
    // JSON benchmarks
    // JSON string to runtime value
    let json = get_json_test_string(file_path);
    let dxb  = compile_script(&json).expect("Failed to parse JSON string");
    // serde
    c.bench_with_input(BenchmarkId::new("json to runtime value", file_path), &json, |b, json| {
        b.iter(|| {
            black_box(json_to_runtime_value_baseline_serde(black_box(json)));
        })
    });
    // json_syntax
    c.bench_with_input(BenchmarkId::new("json_syntax to runtime value", file_path), &json, |b, json| {
        b.iter(|| {
            black_box(json::json_to_runtime_value_baseline_json_syntax(black_box(json)));
        })
    });
    // DATEX
    c.bench_with_input(BenchmarkId::new("datex to runtime value", file_path), &json, |b, json| {
        b.iter(|| {
            black_box(json::json_to_runtime_value_datex(black_box(json)));
        })
    });
    // JSON string to DXB
    c.bench_with_input(BenchmarkId::new("json to DXB", file_path), &json, |b, json| {
        b.iter(|| {
            black_box(json_to_dxb(black_box(json)));
        })
    });
    // DXB
    c.bench_with_input(BenchmarkId::new("dxb to runtime value", file_path), &dxb, |b, dxb| {
        b.iter(|| {
            black_box(json::dxb_to_runtime_value(black_box(dxb)));
        })
    });


    // runtime value to JSON
    let json_serde = json::json_to_serde_value(&json);
    let json_syntax = json::json_to_json_syntax_value(&json);
    let json_datex = json::json_to_datex_value(&json);
    // serde
    c.bench_with_input(BenchmarkId::new("runtime value to serde JSON", file_path), &json_serde, |b, json_serde| {
        b.iter(|| {
            black_box(json::runtime_value_to_json_baseline_serde_json(black_box(json_serde)));
        })
    });
    // json_syntax
    c.bench_with_input(BenchmarkId::new("runtime value to json_syntax JSON", file_path), &json_syntax, |b, json_syntax| {
        b.iter(|| {
            black_box(json::runtime_value_to_json_baseline_json_syntax(black_box(json_syntax)));
        })
    });
    // DATEX
    c.bench_with_input(BenchmarkId::new("runtime value to DATEX JSON", file_path), &json_datex, |b, json_datex| {
        b.iter(|| {
            black_box(json::runtime_value_to_json_datex(black_box(json_datex)));
        })
    });
    // DXB
    c.bench_with_input(BenchmarkId::new("runtime value to DXB", file_path), &json_datex, |b, json_datex| {
        b.iter(|| {
            black_box(json::runtime_value_to_dxb(black_box(json_datex)));
        })
    });
    // DXB to JSON
    c.bench_with_input(BenchmarkId::new("DXB to JSON", file_path), &dxb, |b, dxb| {
        b.iter(|| {
            black_box(json::dxb_to_json(black_box(dxb)));
        })
    });
}

fn bench_json(c: &mut Criterion) {
    bench_json_file(c, "test1.json");
    bench_json_file(c, "test2.json");
}

criterion_group!{name = json; config = Criterion::default().sample_size(10); targets = bench_json}
criterion_group!(runtime, bench_runtime);

criterion_main!(json);
