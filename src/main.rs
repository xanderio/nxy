use clap::{Parser, Subcommand};
use color_eyre::{eyre::Context, Result};
use sqlx::postgres::PgPoolOptions;
use tracing::instrument;

mod profile;

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

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    install_tracing();
    color_eyre::install()?;

    let database_url = std::env::var("DATABASE_URL").wrap_err("DATABASE_URL unset")?;
    let pool = PgPoolOptions::new().connect(&database_url).await?;
    sqlx::migrate!().run(&pool).await?;

    let opts = Opts::parse();
    match opts.action {
        Action::List { target } => list_profiles(&target)?,
    };

    Ok(())
}

#[instrument]
fn list_profiles(flake: &str) -> Result<()> {
    let deploy = crate::profile::load_deployment_metadata(&[flake])?;

    print!("{deploy}");
    Ok(())
}

fn install_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();
}
