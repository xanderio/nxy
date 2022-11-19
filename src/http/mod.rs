mod agent;
mod error;
mod flakes;

use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use axum::{Router, RouterService};
use color_eyre::eyre::WrapErr;
use sqlx::PgPool;
use tower_http::trace::TraceLayer;

use crate::{agent::AgentManager, http::error::Error};

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub(crate) struct ApiContext {
    db: PgPool,
    agent_manager: Arc<AgentManager>,
}

pub async fn serve(db: PgPool, agent_manager: Arc<AgentManager>) -> color_eyre::Result<()> {
    let api_context = ApiContext { db, agent_manager };

    let app = api_router(api_context);

    //TODO: make port configuratable
    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 8080));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .wrap_err("error running HTTP server")
}

fn api_router(api_context: ApiContext) -> RouterService {
    Router::new()
        .merge(flakes::router())
        .merge(agent::router())
        // Enable logging. Use `RUST_LOG=tower_http=debug`
        .layer(TraceLayer::new_for_http())
        .with_state(api_context)
}
