# DATEX Core

This repository contains a full DATEX Runtime & Compiler/Decompiler, written in Rust.

## Building

Required rust version: nightly-2022-12-12
Required for generators feature (renamed to coroutines in newer nightly versions, but not yet updated in gen-iter)

## Testing

The integration tests in the test/ directory can be run with `cargo test -- --show-output`