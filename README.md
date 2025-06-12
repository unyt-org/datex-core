# DATEX Core

[![Twitter badge][]][Twitter link] [![Discord badge][]][Discord link]

<img align="right" src="./.github/assets/datex-logo-light.svg" height="150px">

This repository contains a full DATEX Runtime, Compiler and Decompiler, written in Rust.
The DATEX Core crate is used in [DATEX Core JS](https://github.com/unyt-org/datex-core-js), 
which provides a JavaScript interface to the DATEX Runtime.
The [DATEX CLI](https://github.com/unyt-org/datex-cli) is also built on top of this crate and provides a command line interface for the DATEX Runtime.


## Development

### Running Tests

Tests must be run with the `debug` feature enabled. You can either run the tests with
`cargo test --features debug` or use the alias `cargo test-debug`.

### Clippy
To apply clippy fixes, run the following command:

```bash
cargo clippy-debug
```

### Running Benchmarks

The benchmarks in the `benches` directory can be run with the following command:

```bash
cargo bench
```

Benchmarks are also run automatically in the GitHub CI on every push to the main branch or a pull request branch.

## Contributing

We welcome every contribution!<br>
Please take a look at the [development guidelines](./DEVELOP.md) and the
unyt.org [contribution guidlines](https://github.com/unyt-org/.github/blob/main/CONTRIBUTING.md).


[Twitter badge]: https://img.shields.io/twitter/follow/unytorg.svg?style=social&label=Follow

[Twitter link]: https://twitter.com/intent/follow?screen_name=unytorg

[Discord badge]: https://img.shields.io/discord/928247036770390016?logo=discord&style=social

[Discord link]: https://unyt.org/discord
