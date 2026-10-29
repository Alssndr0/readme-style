//! `readme-style`: keep your README's words, upgrade its presentation.

mod args;
mod files;
mod report;
mod run;

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let args = args::Args::parse();
    args.color.write_global();
    run::main(&args)
}
