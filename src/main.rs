use axum::{Extension, Router};
use clap::{Parser, Subcommand};
use color_eyre::{eyre::Context, Result};
use flake::InputFlakeStore;
use futures_util::stream::TryStreamExt;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tower_http::trace::TraceLayer;
use tracing::instrument;

mod flake;
pub mod server;
mod rpc;

#[derive(Debug, Clone, Parser)]
struct Opts {
    #[command(subcommand)]
    action: Action,
}

#[derive(Debug, Clone, Subcommand)]
#[command()]
pub enum Action {
    /// Print all input flakes
    List,
    AddFlake {
        repo_url: String,
    },
    Check,
    Server,
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
        Action::List => list_input_flakes(pool).await?,
        Action::Check => check_for_updates(pool).await?,
        Action::Server => run_server(pool).await?,
        Action::AddFlake { repo_url } => add_flake(repo_url, pool).await?,
    };

    Ok(())
}

async fn add_flake(flake_url: String, pool: PgPool) -> Result<()> {
    let store = InputFlakeStore::new(pool);
    store.get_or_add(flake_url).await?;
    Ok(())
}

#[instrument(skip(pool))]
async fn list_input_flakes(pool: PgPool) -> Result<()> {
    let store = InputFlakeStore::new(pool);
    while let Some(flake) = store.stream().await.try_next().await? {
        println!("{}", flake.flake_url);
    }

    Ok(())
}

#[instrument(skip(pool))]
async fn check_for_updates(pool: PgPool) -> Result<()> {
    let store = InputFlakeStore::new(pool);
    while let Some(flake) = store.stream().await.try_next().await? {
        flake.update().await?;
    }
    Ok(())
}

async fn run_server(pool: PgPool) -> Result<()> {
    let app = Router::new()
        .layer(TraceLayer::new_for_http())
        .layer(Extension(pool))
        .nest("/", server::router());

    axum::Server::bind(&"0.0.0.0:8080".parse()?)
        .serve(app.into_make_service())
        .await
        .map_err(Into::into)
}

fn install_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    let fmt_layer = fmt::layer().pretty();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();
}
