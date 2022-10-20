use color_eyre::Result;
use futures_util::{SinkExt, TryStreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::rpc::{JsonRPC, Request, Response};

pub async fn run() -> Result<()> {
    let (mut ws, _) = connect_async("ws://localhost:8080/ws").await?;

    while let Some(msg) = ws.try_next().await? {
        let rpc: JsonRPC = msg.into_text()?.parse()?;
        match rpc {
            JsonRPC::Request(request) => {
                let res: JsonRPC = handle_request(request).await?.into();
                ws.send(Message::Text(res.to_string())).await?;
            }
            JsonRPC::Response(res) => tracing::warn!(?res, "received response, this should happen"),
            JsonRPC::Notification(notification) => tracing::info!(?notification),
        }
    }
    Ok(())
}

async fn handle_request(request: Request) -> Result<Response> {
    match request.method.as_str() {
        "ping" => handler::ping(&request),
        _ => handler::unknown(&request),
    }
}

mod handler {
    use crate::rpc::{ErrorCode, Request, Response};
    use color_eyre::Result;
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
}
