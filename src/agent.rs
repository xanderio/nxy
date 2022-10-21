use std::{
    collections::HashMap,
    sync::{atomic::AtomicU64, Arc, Mutex},
};

use color_eyre::Result;
use rpc::{JsonRPC, Request, RequestId, Response};
use serde::Serialize;
use tokio::sync::{mpsc, oneshot};
use tracing::{instrument, Level};

#[derive(Debug, Clone)]
pub struct Agent(Arc<AgentInner>);

#[derive(Debug)]
struct AgentInner {
    next_request_id: AtomicU64,
    pending: Mutex<HashMap<RequestId, oneshot::Sender<Response>>>,
    outbox: mpsc::Sender<JsonRPC>,
    span: tracing::Span,
}

impl Agent {
    pub fn new(inbox: mpsc::Receiver<JsonRPC>, outbox: mpsc::Sender<JsonRPC>) -> Self {
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

    pub async fn ping(&self) -> Result<()> {
        let res = self.send_request("ping", ()).await.await?;
        if let Some(error) = res.error {
            Err(color_eyre::eyre::eyre!("request error: {:?}", error))
        } else {
            Ok(())
        }
    }
}
