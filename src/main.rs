use agent::AgentManager;
use color_eyre::{eyre::Context, Result};
use sqlx::{postgres::PgPoolOptions, PgPool};

mod agent;
mod api;
mod error;
mod http;
mod server;

#[tokio::main]
async fn main() -> Result<()> {
    install_tracing();
    color_eyre::install()?;

    let database_url = std::env::var("DATABASE_URL").wrap_err("DATABASE_URL unset")?;
    let pool = PgPoolOptions::new().connect(&database_url).await?;
    sqlx::migrate!().run(&pool).await?;

    run_server(pool).await
}

async fn run_server(pool: PgPool) -> Result<()> {
    let agent_manager = AgentManager::start(pool.clone()).await;
    let app = server::router(pool, agent_manager);

    tracing::info!("running on 0.0.0.0:8080");
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
    let console_layer = console_subscriber::spawn();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(console_layer)
        .with(ErrorLayer::default())
        .init();
}
