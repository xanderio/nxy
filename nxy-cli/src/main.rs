mod args;
mod handler;
mod utils;

use args::{Action, Args};

use clap::Parser;
use color_eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();

    match args.action {
        Action::Agents { action } => handler::agent::handle(action, args.format),
        Action::Flakes { action } => handler::flake::handle(action, args.format),
        Action::Configs { action } => handler::configuration::handle(action, args.format),
    }
}
