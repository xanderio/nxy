mod websocket;

use axum::{routing::get, Router};

use super::ApiContext;

pub(crate) fn router() -> Router<ApiContext> {
    Router::new().route("/api/v1/agent/ws", get(websocket::ws_handler))
}
