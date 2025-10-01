# Release Guide

This document describes the process of releasing a new version of the Datex Core
library. The process creates a Github release, updates the version number in the
Cargo.toml file, and publishes the new version to crates.io.

1. Run the
   ["Create release"](https://github.com/unyt-org/datex-core/actions/workflows/create-release.yml)
   workflow in Github.

- Specify whether you want to create a new major, minor, or patch release.
- The workflow will create a new release branch based on the current main
  branch, named `release/MAJOR.MINOR.PATCH` and also opens a Pull Request to
  merge the release branch into the main branch. The version number in the
  Cargo.toml file is also automatically updated to the new version.

2. Create feat, fix, chore, refactor, docs or test branches from the release
   branch to work on new features, bug fixes, etc.
   - Command to create a new branch:
   ```sh
   git fetch && git checkout -b feat/feature-name release/MAJOR.MINOR.PATCH
   ```
3. Create a Pull Request to merge the branch into the release branch when ready
4. When all features are merged into the release branch, close the Pull Request
   to merge the release branch into the main branch.
5. A new draft release will be created in the Github repository.

- Review the release notes and make any necessary changes
- Publish the release
- A crates.io release will automatically be published
