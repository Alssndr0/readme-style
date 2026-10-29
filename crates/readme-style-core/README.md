# readme-style-core

The engine behind [readme-style](https://github.com/Alssndr0/readme-style): restyle a README's
presentation without changing what it says.

```rust
use readme_style_core::{project::ProjectFacts, style, theme::Themes};

let theme = Themes::builtin().get("startup")?;
let facts = ProjectFacts { license: Some("MIT".into()), ..ProjectFacts::default() };
let outcome = style("# demo\n\n> **Note:** hello\n", &theme, &facts)?;
println!("{}", outcome.output);
```

`style` never returns a README that says something different from its input, and its output is a
fixed point. Project facts come from `project::inspect` (through a `FileSystem`, so it also runs in
memory), themes from `theme::Themes`, configuration from `config::discover`. The command-line tool
is the `readme-style` crate.
