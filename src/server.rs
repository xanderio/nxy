use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Extension, WebSocketUpgrade,
    },
    headers,
    response::IntoResponse,
    routing::get,
    Router, TypedHeader,
};
use color_eyre::Result;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use sqlx::PgPool;
use tokio::sync::mpsc;
use tower_http::trace::TraceLayer;
use tracing::instrument;

use rpc::{ErrorCode, JsonRPC, Response};

use crate::agent::{Agent, AgentManager};

pub fn router(pool: PgPool, agent_manager: Arc<AgentManager>) -> Router {
    Router::new()
        .route("/ws", get(ws_handler))
        .layer(Extension(agent_manager))
        .layer(Extension(pool))
        .layer(TraceLayer::new_for_http())
}

#[instrument(skip_all)]
async fn ws_handler(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    Extension(agent_manager): Extension<Arc<AgentManager>>,
) -> impl IntoResponse {
    if let Some(TypedHeader(user_agent)) = user_agent {
        tracing::info!("`{}` connected", user_agent.as_str());
    }

    ws.on_upgrade(|socket| handle_socket(socket, agent_manager))
}

#[instrument(skip_all)]
async fn handle_socket(socket: WebSocket, agent_manager: Arc<AgentManager>) {
    let (inbox_sender, inbox) = mpsc::channel(4096);
    let (outbox, outbox_receiver) = mpsc::channel(4096);
    let (sink, stream) = socket.split();
    let inbox_handler = tokio::spawn(process_inbox(stream, inbox_sender));
    let outbox_handler = tokio::spawn(process_outbox(sink, outbox_receiver));

    let agent = Agent::new(inbox, outbox);
    agent_manager.add_agent(agent).await.unwrap();

    inbox_handler.await.unwrap().unwrap();
    outbox_handler.await.unwrap();
}

#[instrument(skip_all)]
async fn process_outbox(
    mut sink: SplitSink<WebSocket, Message>,
    mut outbox_receiver: mpsc::Receiver<JsonRPC>,
) {
    while let Some(msg) = outbox_receiver.recv().await {
        sink.send(Message::Text(msg.to_string())).await.unwrap();
    }
}

#[instrument(skip_all)]
async fn process_inbox(
    mut stream: SplitStream<WebSocket>,
    tx: mpsc::Sender<JsonRPC>,
) -> Result<()> {
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
