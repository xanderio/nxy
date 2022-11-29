use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    headers,
    response::IntoResponse,
    TypedHeader,
};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use rpc::{ErrorCode, JsonRPC, Response};
use tokio::sync::mpsc;
use tracing::instrument;

use crate::{agent::Agent, http::ApiContext};

#[instrument(skip_all)]
pub(super) async fn ws_handler(
    ws: WebSocketUpgrade,
    ctx: State<ApiContext>,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
) -> impl IntoResponse {
    if let Some(TypedHeader(user_agent)) = user_agent {
        tracing::info!("`{}` connected", user_agent.as_str());
    }

    ws.on_upgrade(|socket| handle_socket(socket, ctx))
}

#[instrument(skip_all)]
async fn handle_socket(socket: WebSocket, ctx: State<ApiContext>) {
    let (inbox_sender, inbox) = mpsc::channel(4096);
    let (outbox, outbox_receiver) = mpsc::channel(4096);
    let (sink, stream) = socket.split();
    let inbox_handler = tokio::spawn(process_inbox(stream, inbox_sender));
    let outbox_handler = tokio::spawn(process_outbox(sink, outbox_receiver));

    let agent = Agent::new(inbox, outbox);
    ctx.agent_manager.add_agent(agent).await.unwrap();

    inbox_handler.await.unwrap();
    outbox_handler.await.unwrap();
}

#[instrument(skip_all)]
async fn process_outbox(
    mut sink: SplitSink<WebSocket, Message>,
    mut outbox_receiver: mpsc::Receiver<JsonRPC>,
) {
    while let Some(msg) = outbox_receiver.recv().await {
        if let Err(err) = sink.send(Message::Text(msg.to_string())).await {
            tracing::warn!(?err, "connection closed");
            return;
        };
    }
}

#[instrument(skip_all)]
async fn process_inbox(mut stream: SplitStream<WebSocket>, tx: mpsc::Sender<JsonRPC>) {
    while let Some(Ok(msg)) = stream.next().await {
        match msg {
            Message::Text(t) => {
                tracing::debug!("client sent str: {:?}", t);
                let msg = match t.parse() {
                    Ok(rpc) => rpc,
                    Err(err) => {
                        // compiler needs a little help with the type signature
                        let err: rpc::error::Error = err;
                        Response::new_err(
                            // we don't have a request id in this case, the standard allow
                            // that the request id is empty in this case, but our
                            // implementation doesn't support this.
                            u64::MAX.into(),
                            ErrorCode::ParseError as i32,
                            err.to_string(),
                        )
                        .into()
                    }
                };
                if let Err(err) = tx.send(msg).await {
                    tracing::warn!(
                        ?err,
                        "error sending incomming msg to agent, closing connection"
                    );
                    break;
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
                break;
            }
        }
    }
    tracing::debug!("client disconnected");
}
