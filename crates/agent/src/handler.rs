use color_eyre::Result;
use rpc::{ErrorCode, Request, Response};
use tracing::instrument;

#[instrument]
pub(super) fn ping(request: &Request) -> Result<Response> {
    tracing::info!("PONG");
    Ok(Response::new_ok(request.id, "pong"))
}

#[instrument]
pub(super) fn unknown(request: &Request) -> Result<Response> {
    Ok(Response::new_err(
        request.id,
        ErrorCode::MethodNotFound as i32,
        "pong".to_string(),
    ))
}
