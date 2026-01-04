# Troubleshooting

This document provides solutions to common issues you may encounter while
working with this project.

## Rust Compiler gets stuck during compilation without errors

Possible Fixes:

- Check if you made any modifications to structs that derive BinRead/BinWrite
  from `binrw` (e.g. in `src/global/protocol_structures/instructions.rs`). Some
  values are not supported by `binrw` per default, e.g. `bool` values without
  custom mapping. In this case, the compiler may get stuck without providing any
  error messages.
