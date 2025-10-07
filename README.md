# DATEX Core

[![Twitter badge][]][Twitter link] [![Discord badge][]][Discord link]

<img align="right" src="assets/datex-logo-light.svg" width="150px" alt="The DATEX logo">

This repository contains the full DATEX Runtime including networking, compiler
and decompiler, written in Rust. The DATEX Core crate is used in
[DATEX Core JS](https://github.com/unyt-org/datex-core-js), which provides a
JavaScript interface to the DATEX Runtime. The
[DATEX CLI](https://github.com/unyt-org/datex-cli) is also built on top of this
crate and provides a command line interface for the DATEX Runtime.

## Project Structure

- [src/](./src) - Contains the source code of the crate
  - [ast/](./src/ast) - Abstract syntax tree (AST) modules
  - [compiler/](./src/compiler) - Compiler for the DATEX language
  - [crypto/](./src/crypto) - Cryptographic trait and a native implementation
  - [decompiler/](./src/decompiler) - Decompiler for the DATEX language
  - [dif/](./src/dif) - Abstract data interface for data exchange with external
    systems
  - [global/](./src/global) - Global constants and structures
  - [libs/](./src/libs) - Library modules such as core library and standard
    library
  - [network/](./src/network) - Network protocol implementation and
    communication interfaces
  - [parser/](./src/parser) - DXB parser and instruction handler
  - [references/](./src/references) - Reference implementation, observers and
    mutators
  - [runtime/](./src/runtime) - Runtime for executing DXB
  - [serde/](./src/serde) - Serialization and deserialization of DATEX values
  - [traits/](./src/traits) - Shared traits for values, types and references
  - [types/](./src/types) - Type system implementation
  - [utils/](./src/utils) - Utility functions and traits
  - [values/](./src/values) - Value implementation, core values and value
    containers
- [benches/](./benches) - Benchmarks for the crate for performance testing
- [tests/](./tests) - Integration tests for the crate
- [macros/](./macros) - Procedural macros for the crate
- [docs/](./docs) - Documentation for the crate
  - [guide/](./docs/guide) - Collection of guides for contributing to the crate

## Environment

- [DATEX Specification](https://github.com/unyt-org/datex-specification) - The
  specification of DATEX, including protocols, syntax, and semantics. The
  specification is work in progress and is not yet complete. It is being
  developed in parallel with the implementation of the DATEX Core. The
  repository is currently private, but will be made public in the future and is
  available to contributors on [request](https://unyt.org/contact).
- [DATEX Core JS](https://github.com/unyt-org/datex-core-js) - A JavaScript
  interface to the DATEX Core, built on top of this crate. Includes a
  WebAssembly build for running DATEX in the browser or server-side with
  [Deno](https://deno.land/), [Node.js](https://nodejs.org/), and
  [Bun](https://bun.sh/) and trait implementations using standard web APIs such
  as
  [WebCrypto](https://developer.mozilla.org/en-US/docs/Web/API/Web_Crypto_API)
  and [WebSocket](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket).
- [DATEX CLI](https://github.com/unyt-org/datex-cli) - A command line interface
  for the DATEX Core, built on top of this crate. Provides a simple way to run
  DATEX scripts and interact with the DATEX Runtime in a REPL-like environment.
- [DATEX Core ESP32](https://github.com/unyt-org/datex-core-esp32) - A port of
  the DATEX Core to the
  [ESP32](https://www.espressif.com/en/products/socs/esp32) platform, allowing
  you to run DATEX on microcontrollers of the Espressif family.
- [DATEX Core CPP](https://github.com/unyt-org/datex-core-cpp) - A C++ port of
  the DATEX Core, allowing you to run DATEX on platforms that support C++. _This
  port is still in development and not functional._
- [DATEX Core JS (legacy)](https://github.com/unyt-org/datex-core-js-legacy) - A
  legacy version of the DATEX Core JS, implemented in TypeScript. This version
  will be replaced by the new DATEX Core JS implementation.

## Development

### Building the Project

The project is build with Rust Nightly
([`rustc 1.91.0-nightly`](https://releases.rs/docs/1.91.0/)). To build the
project, run:

```bash
cargo build
```

### Running Tests

Tests must be run with the `debug` feature enabled. You can either run the tests
with `cargo test --features debug` or use the alias `cargo test-debug`.

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

Benchmarks are also run automatically in the GitHub CI on every push to the main
branch or a pull request branch.

## Contributing

**We welcome every contribution!**<br> Please take a look at the
[DATEX Core contribution guidelines](./CONTRIBUTING.md) and the unyt.org
[contribution guidlines](https://github.com/unyt-org/.github/blob/main/CONTRIBUTING.md).

[Twitter badge]: https://img.shields.io/twitter/follow/unytorg.svg?style=social&label=Follow
[Twitter link]: https://twitter.com/intent/follow?screen_name=unytorg
[Discord badge]: https://img.shields.io/discord/928247036770390016?logo=discord&style=social
[Discord link]: https://unyt.org/discord

---

<sub>&copy; unyt 2025 • [unyt.org](https://unyt.org)</sub>
