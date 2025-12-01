# Project Architecture

This documents describes the general architecture of the DATEX Core Project, as well as the relationships with other projects in the DATEX ecosystem.

## Overview

The DATEX Core crate contains all fundamental building blocks of the DATEX runtime and language, including
* The DATEX compiler, that compiles DATEX source code into DXB (DATEX binary) format
* The DATEX language server, that provides IDE support for DATEX development
* The DATEX runtime, that executes DXB and handles communication, synchronization, storage etc.
* The DATEX core and std libraries
* Common traits for core communication interfaces (e.g. TCP, UDP, Serial, etc.)
* Default implementations for native platforms of communication interfaces, storage backends, and cryptographic methods

## Other Projects building on DATEX Core

We are currently actively developing several other projects that build on top of DATEX Core:
* **[DATEX Core JS](https://github.com/unyt-org/datex-core-js)**: A library that provides JavaScript bindings for DATEX Core via WebAssembly
* **[DATEX Core Embedded](https://github.com/unyt-org/datex-core-embedded)**: A wrapper around DATEX Core that provides default implementations for embedded targets such as ESP32
* **[DATEX CLI](https://github.com/unyt-org/datex-cli)**: A command-line interface for running DATEX code
