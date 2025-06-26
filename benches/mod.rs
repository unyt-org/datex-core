#![feature(custom_test_frameworks)]
#![test_runner(criterion::runner)]
use crate::json::{
    get_json_test_string, json_to_dxb, json_to_runtime_value_baseline_serde,
};
use crate::runtime::runtime_init;
use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion,
};
use datex_core::compiler::bytecode::{compile_script, CompileOptions};

mod json;
mod runtime;

fn bench_runtime(c: &mut Criterion) {
    c.bench_function("runtime init", |b| b.iter(runtime_init));
}

fn bench_json_file(c: &mut Criterion, file_path: &str) {
    // JSON benchmarks
    // JSON string to runtime value
    let json = get_json_test_string(file_path);
    let (dxb, _) = compile_script(&json, CompileOptions::default())
        .expect("Failed to parse JSON string");

    // serde
    c.bench_with_input(
        BenchmarkId::new("json to runtime value serde_json", file_path),
        &json,
        |b, json| {
            b.iter(|| {
                json_to_runtime_value_baseline_serde(black_box(json));
                black_box(());
            })
        },
    );
    // json_syntax
    c.bench_with_input(
        BenchmarkId::new("json to runtime value json_syntax", file_path),
        &json,
        |b, json| {
            b.iter(|| {
                json::json_to_runtime_value_baseline_json_syntax(black_box(
                    json,
                ));
                black_box(());
            })
        },
    );
    // DATEX
    c.bench_with_input(
        BenchmarkId::new("json to runtime value datex", file_path),
        &json,
        |b, json| {
            b.iter(|| {
                json::json_to_runtime_value_datex(black_box(json), None);
                black_box(());
            })
        },
    );
    // DATEX (automatic static value detection)
    c.bench_with_input(
        BenchmarkId::new(
            "json to runtime value datex auto static detection",
            file_path,
        ),
        &json,
        |b, json| {
            b.iter(|| {
                json::json_to_runtime_value_datex_auto_static_detection(
                    black_box(json),
                    None,
                );
                black_box(());
            })
        },
    );
    // DATEX (forced static value)
    c.bench_with_input(
        BenchmarkId::new(
            "json to runtime value datex forced static",
            file_path,
        ),
        &json,
        |b, json| {
            b.iter(|| {
                json::json_to_runtime_value_datex_force_static_value(
                    black_box(json),
                );
                black_box(());
            })
        },
    );

    // JSON string to DXB
    c.bench_with_input(
        BenchmarkId::new("json to dxb", file_path),
        &json,
        |b, json| {
            b.iter(|| {
                json_to_dxb(black_box(json), None);
                black_box(());
            })
        },
    );
    // DXB
    c.bench_with_input(
        BenchmarkId::new("dxb to runtime value", file_path),
        &dxb,
        |b, dxb| {
            b.iter(|| {
                json::dxb_to_runtime_value(black_box(dxb));
                black_box(());
            })
        },
    );

    // runtime value to JSON
    let json_serde = json::json_to_serde_value(&json);
    let json_syntax = json::json_to_json_syntax_value(&json);
    let json_datex = json::json_to_datex_value(&json);
    // serde
    c.bench_with_input(
        BenchmarkId::new("runtime value to json serde_json", file_path),
        &json_serde,
        |b, json_serde| {
            b.iter(|| {
                json::runtime_value_to_json_baseline_serde_json(black_box(
                    json_serde,
                ));
                black_box(());
            })
        },
    );
    // json_syntax
    c.bench_with_input(
        BenchmarkId::new("runtime value to json json_syntax", file_path),
        &json_syntax,
        |b, json_syntax| {
            b.iter(|| {
                json::runtime_value_to_json_baseline_json_syntax(black_box(
                    json_syntax,
                ));
                black_box(());
            })
        },
    );
    // DATEX
    c.bench_with_input(
        BenchmarkId::new("runtime value to json datex", file_path),
        &json_datex,
        |b, json_datex| {
            b.iter(|| {
                json::runtime_value_to_json_datex(black_box(json_datex));
                black_box(());
            })
        },
    );
    // DXB
    c.bench_with_input(
        BenchmarkId::new("runtime value to dxb", file_path),
        &json_datex,
        |b, json_datex| {
            b.iter(|| {
                json::runtime_value_to_dxb(black_box(json_datex));
                black_box(());
            })
        },
    );
    // DXB to JSON
    c.bench_with_input(
        BenchmarkId::new("dxb to json", file_path),
        &dxb,
        |b, dxb| {
            b.iter(|| {
                json::dxb_to_json(black_box(dxb));
                black_box(());
            })
        },
    );
}

fn bench_json(c: &mut Criterion) {
    bench_json_file(c, "test1.json");
    bench_json_file(c, "test2.json");
    bench_json_file(c, "test3.json");
}

criterion_group! {name = json; config = Criterion::default().sample_size(10); targets = bench_json}
criterion_group!(runtime, bench_runtime);

criterion_main!(json);
