use std::{
    collections::HashMap,
    sync::{atomic::AtomicU64, Arc, Mutex},
    time::Duration,
};

use color_eyre::{eyre::eyre, Result};
use nxy_rpc::{types::Status, JsonRPC, Request, RequestId, Response};
use serde::Serialize;
use sqlx::PgPool;
use tokio::sync::{mpsc, oneshot};
use tracing::{instrument, Level};
use uuid::Uuid;

#[derive(Debug)]
pub struct AgentManager {
    pool: PgPool,
    agents: Mutex<HashMap<Uuid, Agent>>,
}

impl AgentManager {
    pub async fn start(pool: PgPool) -> Arc<Self> {
        let manager = Arc::new(Self {
            pool,
            agents: Default::default(),
        });

        let manager_c = Arc::clone(&manager);
        tokio::spawn(async move { manager_c.heartbeat().await });

        manager
    }

    pub async fn heartbeat(&self) {
        loop {
            let agents = self.agents.lock().unwrap().clone();
            for agent in agents.values() {
                agent.ping().await.unwrap();
            }
            tokio::time::sleep(Duration::from_secs(5)).await
        }
    }

    pub async fn add_agent(&self, agent: Agent) -> Result<()> {
        // request agent status to aquire the agent id
        let status = agent.status().await?;

        let result =
            sqlx::query_scalar!("SELECT agent_id FROM agents WHERE agent_id = $1", status.id)
                .fetch_optional(&self.pool)
                .await?;

        if result.is_none() {
            tracing::info!(id = ?status.id, "new agent established a connection");
            sqlx::query!("INSERT INTO agents (agent_id) VALUES ($1)", status.id)
                .execute(&self.pool)
                .await?;
        } else {
            tracing::info!(id = ?status.id, "known agent connected");
        }
        sqlx::query!(
            "UPDATE agents SET current_system = $2 WHERE agent_id = $1",
            status.id,
            status.system.current.to_str().unwrap()
        )
        .execute(&self.pool)
        .await?;

        {
            let mut agents = self.agents.lock().unwrap();
            agents.insert(status.id, agent);
        }
        Ok(())
    }
}

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
        let res = self.send_request("$/ping", ()).await.await?;
        if let Some(error) = res.error {
            Err(eyre!("request error: {:?}", error))
        } else {
            Ok(())
        }
    }

    pub async fn status(&self) -> Result<Status> {
        let res = self.send_request("$/status", ()).await.await?;
        if let Some(error) = res.error {
            Err(eyre!("request error: {:?}", error))
        } else {
            res.result
                .ok_or_else(|| eyre!("status result is empty"))
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        }
    }
}
