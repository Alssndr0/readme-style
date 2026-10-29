//! `readme-style`: keep your README's words, upgrade its presentation.

mod args;
mod files;
mod report;
mod run;
mod suggest;

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let args = args::Args::parse();
    args.color.write_global();
    match &args.command {
        Some(args::Command::Suggest(suggest)) => suggest::main(suggest),
        None => run::main(&args),
    }
}
