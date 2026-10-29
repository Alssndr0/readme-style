# Contributing

Thanks for helping. This file covers the setup, the tests and the usual changes.
[docs/architecture.md](docs/architecture.md) explains how the code fits together.

## Setup

Install Rust with [rustup](https://rustup.rs). The toolchain is pinned in `rust-toolchain.toml`;
the minimum supported version is 1.89.

```bash
cargo test --workspace          # every test
cargo clippy --workspace --all-targets
cargo fmt --all
cargo run -- --help             # the CLI from source
```

## Tests

| Command | What it runs |
|---------|--------------|
| `cargo test --workspace` | Unit, transform, guarantee (300 generated READMEs), gallery, CLI and `suggest` (mock server) tests. |
| `PROPTEST_CASES=10000 cargo test --release -p readme-style-core --test guarantees` | The property tests at CI strength. |
| `cargo xtask examples` | Regenerates `examples/*/README.<theme>.md`; review the diff before committing. |
| `cargo xtask corpus fetch` then `cargo xtask corpus run` | Every theme on about 140 popular READMEs, cloned into `target/corpus`. |
| `cargo xtask debug PATH THEME [THEME...]` | Styles without checks and shows what a second pass would change. |
| `cargo xtask screenshot README.md out.png` | Renders through GitHub's API and screenshots it (needs `gh` and Chrome). |
| `cargo xtask gallery owner__name:theme ...` | Before and after screenshots of corpus READMEs, side by side in `target/gallery/index.html`. |

The corpus run must report no refusals, no instability and no theme-switch drift. When a property
test fails, proptest prints the smallest failing README; add it as a regression test once fixed.

## Common changes

### A badge

1. Add the kind to `BadgeKind` and to a group in `crates/readme-style-core/src/badges/catalog.rs`.
2. Infer it in `badges/infer.rs`, with the evidence in `reason`. A badge without evidence is a bug:
   prefer showing nothing to showing a broken badge.
3. Add a unit test, and document it in `docs/configuration.md`.

### A theme

See [docs/themes.md](docs/themes.md#adding-a-built-in-theme).

### A transform

1. Add a module under `crates/readme-style-core/src/transform/` and a step in `PIPELINE`; the
   order is explained in the architecture document.
2. Generated content goes in a region (`sync_region`) or on wrapper lines; never edit user content
   except to upgrade its presentation in a way the guard accepts.
3. Add tests in `tests/transforms.rs`, then run the property tests and the corpus.

## Pull requests

- Keep a change to one concern, with tests.
- `cargo fmt`, `cargo clippy` and `cargo test` must pass; CI also runs the property tests on
  Linux, macOS and Windows, the minimum Rust version and `cargo deny`.
- Update `CHANGELOG.md` under `Unreleased` for anything a user would notice.
- Write commit messages in the imperative mood ("Fold FAQ sections"), with the why in the body.
