use clap::{Parser, Subcommand};
use color_eyre::Result;

#[derive(Debug, Clone, Parser)]
struct Opts {
    #[command(subcommand)]
    action: Action,
}

#[derive(Debug, Clone, Subcommand)]
#[command()]
pub enum Action {
    /// Print all nodes and profiles
    List {
        /// The flake to deploy
        #[arg(group = "deploy", default_value = ".")]
        target: String,
    },
}

fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt().pretty().init();

    let opts = Opts::parse();
    match opts.action {
        Action::List { target } => list_profiles(&target)?,
    };

    Ok(())
}

fn list_profiles(flake: &str) -> Result<()> {
    let deploy = deploy::profile::load_deployment_metadata(&[flake])?;

    print!("{deploy}");
    Ok(())
}
