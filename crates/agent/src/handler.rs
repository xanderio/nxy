use std::{io, path::PathBuf};

use color_eyre::Result;
use rpc::{
    types::{Status, System},
    ErrorCode, Request, Response,
};
use serde_json::json;
use tracing::instrument;

#[instrument(skip(request))]
pub(super) fn ping(request: &Request) -> Result<Response> {
    tracing::info!("PONG");
    Ok(Response::new_ok(request.id, "pong"))
}

async fn current_system() -> io::Result<PathBuf> {
    tokio::fs::read_link("/run/current-system").await
}

async fn booted_system() -> io::Result<PathBuf> {
    tokio::fs::read_link("/run/booted-system").await
}

#[instrument(skip(request))]
pub(super) async fn status(request: &Request) -> Result<Response> {
    let system = System {
        current: current_system().await?,
        booted: booted_system().await?,
    };
    let status = Status {
        version: env!("CARGO_PKG_VERSION").to_string(),
        system,
    };

    Ok(Response::new_ok(request.id, json!(status)))
}

#[instrument(skip(request))]
pub(super) fn unknown(request: &Request) -> Result<Response> {
    Ok(Response::new_err(
        request.id,
        ErrorCode::MethodNotFound as i32,
        "pong".to_string(),
    ))
}
