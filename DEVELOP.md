## Building

Required rust version: nightly-2022-12-12
Required for generators feature (renamed to coroutines in newer nightly versions, but not yet updated in gen-iter)


```
rustup install nightly-2022-12-12
rustup default nightly-2022-12-12
```

## Testing

The integration tests in the test/ directory can be run with `cargo test -- --show-output`

```
cargo test --package datex-core --test compiler -- compile_literals --exact --nocapture 
```