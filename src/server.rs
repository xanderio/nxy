use std::{
    collections::HashMap,
    sync::{atomic::AtomicU64, Arc, Mutex},
};

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
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde::Serialize;
use tokio::sync::{mpsc, oneshot};
use tracing::{instrument, Level};

use rpc::{ErrorCode, JsonRPC, Request, RequestId, Response};

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
    let (inbox_sender, inbox) = mpsc::channel(4096);
    let (outbox, outbox_receiver) = mpsc::channel(4096);
    let (sink, stream) = socket.split();
    let inbox_handler = tokio::spawn(process_inbox(stream, inbox_sender));
    let outbox_handler = tokio::spawn(process_outbox(sink, outbox_receiver));

    let agent = Agent::new(inbox, outbox);
    agent.ping().await.unwrap();

    inbox_handler.await.unwrap().unwrap();
    outbox_handler.await.unwrap();
}

async fn process_outbox(
    mut sink: SplitSink<WebSocket, Message>,
    mut outbox_receiver: mpsc::Receiver<JsonRPC>,
) {
    while let Some(msg) = outbox_receiver.recv().await {
        sink.send(Message::Text(msg.to_string())).await.unwrap();
    }
}

#[instrument(skip(stream, tx))]
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

#[derive(Debug, Clone)]
struct Agent(Arc<AgentInner>);

#[derive(Debug)]
struct AgentInner {
    next_request_id: AtomicU64,
    pending: Mutex<HashMap<RequestId, oneshot::Sender<Response>>>,
    outbox: mpsc::Sender<JsonRPC>,
    span: tracing::Span,
}

impl Agent {
    fn new(inbox: mpsc::Receiver<JsonRPC>, outbox: mpsc::Sender<JsonRPC>) -> Self {
        let span = tracing::span!(Level::TRACE, "agent connection");
        let agent = Agent(Arc::new(AgentInner {
            next_request_id: AtomicU64::new(0),
            pending: Default::default(),
            outbox,
            span,
        }));

        let clone = agent.clone();
        tokio::spawn(async move { clone.process_inbox(inbox).await });
        agent
    }

    #[instrument(parent = &self.0.span, skip(self, inbox))]
    async fn process_inbox(self, mut inbox: mpsc::Receiver<JsonRPC>) {
        while let Some(msg) = inbox.recv().await {
            tracing::trace!(?msg, "receiver message");
            match msg {
                JsonRPC::Request(request) => {
                    tracing::warn!(?request, "server received request, this should happen");
                }
                JsonRPC::Response(res) => {
                    tracing::info!("{res:?}");

                    let mut pending = self.0.pending.lock().unwrap();
                    if let Some(tx) = pending.remove(&res.id) {
                        tx.send(res).unwrap();
                    } else {
                        tracing::warn!(
                            request_id = ?res.id,
                            "received response for unknown request id"
                        )
                    }
                }
                JsonRPC::Notification(notification) => {
                    tracing::warn!(
                        ?notification,
                        "server received notification, this should happen"
                    );
                }
            }
        }
    }

    async fn send_request<S: AsRef<str>, P: Serialize>(
        &self,
        method: S,
        params: P,
    ) -> oneshot::Receiver<Response> {
        let id = self
            .0
            .next_request_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            .into();
        let request: JsonRPC = Request::new(id, method, params).into();

        let (sender, receiver) = oneshot::channel();

        {
            let mut pending = self.0.pending.lock().unwrap();
            pending.insert(id, sender);
        }

        self.0.outbox.send(request).await.unwrap();
        receiver
    }

    async fn ping(&self) -> Result<()> {
        let res = self.send_request("ping", ()).await.await?;
        if let Some(error) = res.error {
            Err(color_eyre::eyre::eyre!("request error: {:?}", error))
        } else {
            Ok(())
        }
    }
}
