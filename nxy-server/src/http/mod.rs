mod agent;
mod error;
mod flakes;
mod nixos_configuration;

use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use axum::Router;
use color_eyre::eyre::WrapErr;
use sqlx::PgPool;
use tower_http::trace::TraceLayer;

use crate::{agent::AgentManager, config::Config, http::error::Error};

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub(crate) struct ApiContext {
    config: Arc<Config>,
    db: PgPool,
    agent_manager: Arc<AgentManager>,
}

pub async fn serve(
    config: Arc<Config>,
    db: PgPool,
    agent_manager: Arc<AgentManager>,
) -> color_eyre::Result<()> {
    let api_context = ApiContext {
        config,
        db,
        agent_manager,
    };

    let app = api_router(api_context);

    //TODO: make port configuratable
    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 8085));
    tracing::info!("running on {addr}");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .wrap_err("error running HTTP server")
}

fn api_router(api_context: ApiContext) -> Router<()> {
    Router::new()
        .merge(flakes::router())
        .merge(agent::router())
        .merge(nixos_configuration::router())
        // Enable logging. Use `RUST_LOG=tower_http=debug`
        .layer(TraceLayer::new_for_http())
        .with_state(api_context)
}
