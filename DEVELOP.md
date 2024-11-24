## Building

Rust nightly is required for coroutines.

```sh
rustup install nightly
rustup default nightly
```

## Testing

The integration tests in the test/ directory can be run with `cargo test -- --show-output`

```
cargo test --package datex-core --test compiler -- compile_literals --exact --nocapture 
```