use std::{io, path::PathBuf};

use color_eyre::{
    eyre::{bail, ensure},
    Result,
};
use nxy_common::{
    types::{DownloadParams, Status, System},
    ErrorCode, Request, Response,
};
use serde_json::json;
use tokio::process::Command;
use tracing::instrument;

use crate::STATE;

#[instrument(skip(request))]
pub(super) fn ping(request: &Request) -> Result<Response> {
    tracing::trace!("PONG");
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
    let id = {
        let state = STATE.lock().unwrap();
        state.id
    };
    let status = Status {
        id,
        version: env!("CARGO_PKG_VERSION").to_string(),
        system,
    };

    Ok(Response::new_ok(request.id, json!(status)))
}

pub(super) async fn download(request: &Request) -> Result<Response> {
    let params: DownloadParams = serde_json::from_value(request.params.clone())?;

    let mut cmd = Command::new("nix");
    cmd.args([
        "copy",
        "--substitute-on-destination",
        "--verbose",
        "--no-check-sigs",
        "--from",
        &params.from,
    ]);
    cmd.arg(params.store_path);

    let output = cmd.output().await?;
    if !output.status.success() {
        println!("{}", String::from_utf8_lossy(&output.stderr));
        bail!("nix copy failed");
    }

    Ok(Response::new_ok(request.id, ()))
}

#[instrument(skip(request))]
pub(super) fn unknown(request: &Request) -> Result<Response> {
    Ok(Response::new_err(
        request.id,
        ErrorCode::MethodNotFound as i32,
        "pong".to_string(),
    ))
}
