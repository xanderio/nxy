use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    headers,
    response::IntoResponse,
    routing::get,
    Router, TypedHeader,
};
use color_eyre::Result;
use futures_util::{stream::SplitStream, SinkExt, StreamExt};
use tokio::sync::mpsc::Sender;
use tracing::instrument;

use crate::rpc::{self, ErrorCode, JsonRPC, Response};

pub fn router() -> Router {
    Router::new().route("/ws", get(ws_handler))
}

#[instrument]
async fn ws_handler(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
) -> impl IntoResponse {
    if let Some(TypedHeader(user_agent)) = user_agent {
        tracing::info!("`{}` connected", user_agent.as_str());
    }

    ws.on_upgrade(handle_socket)
}

#[instrument(skip(socket))]
async fn handle_socket(socket: WebSocket) {
    let (tx, mut rx) = tokio::sync::mpsc::channel(4096);
    let (mut sink, stream) = socket.split();
    let receiver = tokio::spawn(process(stream, tx));

    while let Some(msg) = rx.recv().await {
        tracing::info!(?msg, "receiver message");
        match msg {
            JsonRPC::Request(req) => {
                tracing::warn!(?req, "server received request, this should happen");
            }
            JsonRPC::Response(res) => {
                tracing::info!("{res:?}");
            }
            JsonRPC::Notification(_) => {
                let rpc: JsonRPC = rpc::Request::new(1.into(), "ping".to_string(), ()).into();
                sink.send(Message::Text(rpc.to_string())).await.unwrap();
            }
        }
    }
    receiver.await.unwrap().unwrap();
}

#[instrument(skip(stream, tx))]
async fn process(mut stream: SplitStream<WebSocket>, tx: Sender<JsonRPC>) -> Result<()> {
    while let Some(msg) = stream.next().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(t) => {
                    tracing::debug!("client sent str: {:?}", t);
                    match t.parse() {
                        Ok(rpc) => tx.send(rpc).await?,
                        Err(err) => {
                            tx.send(
                                Response::new_err(
                                    0.into(),
                                    ErrorCode::ParseError as i32,
                                    err.to_string(),
                                )
                                .into(),
                            )
                            .await?
                        }
                    };
                }
                Message::Binary(_) => {
                    tracing::warn!(
                        "client sent binary data, this is not supported. Closing connection"
                    );
                    break;
                }
                // ignore ping and pong axum handles this for us
                Message::Ping(_) | Message::Pong(_) => {}
                Message::Close(_) => {
                    tracing::info!("client disconnected");
                    break;
                }
            }
        } else {
            tracing::info!("client disconnected");
            break;
        }
    }
    Ok(())
}
