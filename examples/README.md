# Examples

Three fictional projects, each with its original `README.md` and one styled file per theme. They
are also snapshot tests: `crates/readme-style-core/tests/examples.rs` fails when a change to
readme-style alters any of them, and `cargo xtask examples` regenerates them.

The projects do not exist, so their dynamic badges (versions, CI status) render as "not found".

| Example | Original | minimal | startup | ml-research | cli | docs |
|---------|----------|---------|---------|-------------|-----|------|
| Python library with a changelog, a logo pair and a release workflow | [README](python-lib/README.md) | [view](python-lib/README.minimal.md) | [view](python-lib/README.startup.md) | [view](python-lib/README.ml-research.md) | [view](python-lib/README.cli.md) | [view](python-lib/README.docs.md) |
| Research code with arXiv, Hugging Face and Colab links and a news section | [README](ml-paper/README.md) | [view](ml-paper/README.minimal.md) | [view](ml-paper/README.startup.md) | [view](ml-paper/README.ml-research.md) | [view](ml-paper/README.cli.md) | [view](ml-paper/README.docs.md) |
| Node CLI with a setext title, a FAQ and a Discord link | [README](node-cli/README.md) | [view](node-cli/README.minimal.md) | [view](node-cli/README.startup.md) | [view](node-cli/README.ml-research.md) | [view](node-cli/README.cli.md) | [view](node-cli/README.docs.md) |
