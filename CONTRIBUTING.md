# Contributing to **DATEX Core**

This document describes the workflow, branch strategy, coding standards, and
quality gates for contributing to
the [`datex-core`](https://github.com/unyt-org/datex-core) Rust crate.

---

## Workflow & Branch Strategy

| Purpose                        | Naming Pattern                    | Example                 |
| ------------------------------ | --------------------------------- | ----------------------- |
| **Permanent default branch**   | `main`                            | —                       |
| **Milestone / release branch** | `release/<MAJOR>.<MINOR>.<PATCH>` | `release/0.0.4`         |
| **Feature branch**             | `feature/<slug>`                  | `feature/tcp-interface` |
| **Bug-fix branch**             | `fix/<slug>`                      | `fix/handshake-timeout` |
| **Maintenance / chore branch** | `chore/<slug>`                    | `chore/update-deps`     |

1. **`main` is protected** – direct pushes are disabled.
2. **All work happens via Pull Requests (PRs).**
3. **Target branch for every PR is the currently-active release branch** (e.g.
   `release/0.0.4`).
4. After review & CI success, the feature branch is **squash-merged** into the
   release branch.
5. Release branches are merged back to `main` only by a maintainer at version
   cut-time.

---

## Coding Style

- **Edition:** Rust 2024.

- **Formatting:**

  ```bash
  cargo clippy-debug
  ```

  CI will fail if any file is not properly formatted.

- **Linting:**

  ```bash
  cargo clippy --features debug
  ```

  _We plan to treat all Clippy warnings as errors in the future._ Suppress a
  lint only with a line-level `#[allow(lint_name)]` and an explanatory comment.

- **Idioms & Practices:**

  - Prefer explicit `use` paths; group imports by crate.
  - Enable useful nightly lints in `#![deny(clippy::pedantic, clippy::nursery)]`
    where feasible.
  - No `unsafe` unless unavoidable - must include a safety comment explaining
    invariants.
  - Public items require rustdoc comments (`///`) with examples where possible.
  - Follow **snake\_case** for variables/functions, **CamelCase** for
    types/traits, **SCREAMING\_SNAKE\_CASE** for constants.

---

## Testing & Benchmarking

### Unit Tests

- Each module declares its own **unit tests** inside an internal `tests`
  sub-module:

  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      // …
  }
  ```

- Every public function or logically-independent unit must have at least one
  positive and one negative test.

### Integration Tests

- Mirror the `src/` tree under `tests/`:

  ```
  src/
    crypto/
      mod.rs
      random.rs
  tests/
    crypto/
      random.rs
  ```

- Name the file after the feature being exercised (`network.rs`,
  `persistence.rs`, etc.).

- Integration tests may use the public API only (no `pub(crate)` work-arounds).

### Benchmarks

- Place Criterion benchmarks in `benches/`.
- Benchmarks must compile and run (CI executes them with `--bench` but does not
  time-gate results).
- Performance regressions > 10 % should be called out in the PR description.

---

## Continuous Integration Gates

A pull request is **merge-ready** only when:

1. All unit tests pass: `cargo test --all`.
2. All integration tests pass: `cargo test --all --tests`.
3. All benchmarks build: `cargo bench --no-run`.
4. Clippy passes with no errors
5. Rustfmt check passes
6. Checks complete on all supported toolchains (currently stable, beta).

CI will automatically block a PR that fails any step.

---

## Pull Request Checklist

Before requesting review, ensure you have:

- [ ] Followed the branch naming convention.
- [ ] Rebased onto the latest _active_ release branch.
- [ ] Added/updated unit tests and, if applicable, integration tests.
- [ ] Confirmed `cargo fmt`, `cargo clippy-debug`, **and** all tests/benches
      succeed locally.
- [ ] Updated documentation & examples.
- [ ] Written a clear PR title and description (what, why, how).

---

## Commit & PR Hygiene

- Use **Conventional Commits** style (e.g. `feat: add TCP interface`,
  `fix: handle timeout`).
- Keep commit history clean; squash or amend while the PR is open.
- Reference issues in the PR body (e.g. `Closes #42`).

---

## Communication

- Small changes (< 30 LoC) may be approved by one maintainer; larger or
  architectural changes require two approvals.
- Discuss API-breaking changes in a GitHub Issue before coding.
- Feel free to draft a PR early (`[WIP]`) to get feedback on direction.

---

## Getting Started Locally

```bash
git clone https://github.com/unyt-org/datex-core.git
cd datex-core
rustup override set nightly
cargo test-debug
cargo clippy --features debug
```

You are now ready to create your feature branch and start contributing. Thank
you for helping us shape the future of the unyt.org ecosystem!
