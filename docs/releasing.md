# Releasing

A release is a version tag. `.github/workflows/release.yml` then builds and publishes everything.

## One-time setup

| Where | What |
|-------|------|
| GitHub environment `pypi` | PyPI trusted publishing for `readme-style`, with this repository and `release.yml`. |
| GitHub environment `crates-io` | Secret `CARGO_REGISTRY_TOKEN`, a crates.io token with publish rights. |
| GitHub environment `npm` | Secret `NPM_TOKEN`; the npm organization `readme-style` must exist (platform packages are `@readme-style/<platform>`). |
| Repository settings | Actions may create releases (the default `GITHUB_TOKEN` has `contents: write` in the jobs that need it). |

Protect the three environments with required reviewers so a tag alone cannot publish.

## Each release

1. Update `CHANGELOG.md`: move the `Unreleased` entries under `## [X.Y.Z] - YYYY-MM-DD`.
2. Set `version` in the root `Cargo.toml` (`[workspace.package]`) and the `readme-style-core`
   version under `[workspace.dependencies]`, then `cargo check` to update `Cargo.lock`.
3. Commit, then `git tag vX.Y.Z && git push origin main vX.Y.Z`.

The workflow checks that the tag matches the crate version, creates the GitHub release from the
changelog, and in parallel:

- builds `readme-style-<target>` archives with SHA-256 checksums for Linux (musl), macOS and Windows
  on x86_64 and arm64, attached to the release (the GitHub Action downloads these);
- builds wheels with maturin (manylinux, musllinux, macOS, Windows) and an sdist, and publishes
  them to PyPI;
- publishes `readme-style-core`, then `readme-style`, to crates.io;
- builds the npm platform packages from the release archives (`npm/build.mjs`) and publishes them,
  then the `readme-style` wrapper.

After the first release, point the README's Action and pre-commit examples at the tag, and remove
the "not released yet" note and the badge exclusion in `.readme-style.toml`.
