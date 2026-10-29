//! Time each stage of styling one README: `cargo run --release --example profile -- README.md [theme]`.

#![allow(clippy::print_stdout, missing_docs)]

use std::path::Path;
use std::time::Instant;

use readme_style_core::project::{OsFs, Overrides, inspect};
use readme_style_core::theme::Themes;

fn main() {
    let path = std::env::args().nth(1).expect("a README path");
    let theme = std::env::args().nth(2).unwrap_or_else(|| "startup".into());
    let text = std::fs::read_to_string(&path).expect("readable");
    let path = std::fs::canonicalize(Path::new(&path)).expect("exists");
    let theme = Themes::builtin().get(&theme).expect("theme");
    let start = Instant::now();
    let facts = inspect(&OsFs, &path, &Overrides::default());
    println!("inspect   {:>8.2} ms", start.elapsed().as_secs_f64() * 1000.0);
    let start = Instant::now();
    let once = readme_style_core::style_unchecked(&text, &theme, &facts);
    println!("pipeline  {:>8.2} ms", start.elapsed().as_secs_f64() * 1000.0);
    let start = Instant::now();
    readme_style_core::guard::verify(&text, &once).expect("guard");
    println!("guard     {:>8.2} ms", start.elapsed().as_secs_f64() * 1000.0);
    let start = Instant::now();
    let _ = readme_style_core::style(&text, &theme, &facts).expect("style");
    println!(
        "style     {:>8.2} ms (pipeline twice + guard)",
        start.elapsed().as_secs_f64() * 1000.0
    );
    let start = Instant::now();
    let options = comrak::Options::default();
    let _ = comrak::markdown_to_html(&text, &options);
    println!(
        "comrak    {:>8.2} ms (one render, for scale)",
        start.elapsed().as_secs_f64() * 1000.0
    );
}
