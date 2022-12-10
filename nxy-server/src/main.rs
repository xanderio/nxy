use color_eyre::{eyre::Context, Result};
use nxy_server::agent::AgentManager;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tracing::subscriber::Subscriber;
use tracing_subscriber::Layer;

#[tokio::main]
async fn main() -> Result<()> {
    install_tracing();
    color_eyre::install()?;

    let options = PgConnectOptions::new_without_pgpass();
    let pool = PgPoolOptions::new().connect_with(options).await?;
    sqlx::migrate!().run(&pool).await?;

    let agent_manager = AgentManager::start(pool.clone()).await;

    nxy_server::http::serve(pool, agent_manager).await
}

fn install_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    let fmt_layer = fmt::layer().pretty();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("sqlx=warn,info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(init_console())
        .with(ErrorLayer::default())
        .init();
}

use tracing_subscriber::registry::LookupSpan;

#[cfg(feature = "tokio-console")]
fn init_console<S>() -> impl Layer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    Some(console_subscriber::spawn())
}

#[cfg(not(feature = "tokio-console"))]
fn init_console<S>() -> impl Layer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    None::<Box<dyn Layer<S> + Send + Sync + 'static>>
}
