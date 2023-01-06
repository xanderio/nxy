use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{atomic::AtomicU64, Arc, Mutex},
    time::Duration,
};

use color_eyre::{eyre::eyre, Result};
use nxy_common::{
    types::{DownloadParams, Status},
    JsonRPC, Request, RequestId, Response,
};
use serde::Serialize;
use sqlx::PgPool;
use tokio::sync::{mpsc, oneshot};
use tracing::{instrument, Level};
use uuid::Uuid;

pub(crate) type Inbox = mpsc::Receiver<JsonRPC>;
pub(crate) type Outbox = mpsc::Sender<JsonRPC>;

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

    pub(crate) async fn add_agent(&self, agent: Agent) -> Result<()> {
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

        //XXX: this is a hack and should be replaced with something better.
        match_agent_to_configuration(self.pool.clone()).await?;

        {
            let mut agents = self.agents.lock().unwrap();
            agents.insert(status.id, agent);
        }
        Ok(())
    }

    #[instrument(skip(self))]
    pub(crate) async fn process_update(
        &self,
        config_id: i64,
        flake_revision_id: i64,
    ) -> Result<()> {
        let agent_id = sqlx::query_scalar!(
            "SELECT agent_id FROM agents WHERE nixos_configuration_id = $1",
            config_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(agent_id) = agent_id else { return Ok(()) };

        tracing::info!("Updating configuration on agent {agent_id:?}");

        let store_path = sqlx::query_scalar!(
            "SELECT store_path 
            FROM nixos_configuration_evaluations 
            WHERE flake_revision_id = $1 
                AND nixos_configuration_id = $2",
            flake_revision_id,
            config_id
        )
        .fetch_one(&self.pool)
        .await?;

        let agent = {
            let agents = self.agents.lock().unwrap();
            agents.get(&agent_id).unwrap().clone()
        };

        agent
            .download(DownloadParams {
                store_path: PathBuf::from(store_path),
            })
            .await?;

        Ok(())
    }

    pub(crate) fn get(&self, agent_id: Uuid) -> Option<Agent> {
        let agents = self.agents.lock().unwrap();
        agents.get(&agent_id).cloned()
    }
}

#[derive(Debug, Clone)]
pub struct Agent(Arc<AgentInner>);

#[derive(Debug)]
struct AgentInner {
    next_request_id: AtomicU64,
    pending: Mutex<HashMap<RequestId, oneshot::Sender<Response>>>,
    outbox: Outbox,
    span: tracing::Span,
}

impl Agent {
    pub fn new(inbox: Inbox, outbox: Outbox) -> Self {
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
    async fn process_inbox(self, mut inbox: Inbox) {
        while let Some(msg) = inbox.recv().await {
            tracing::trace!(?msg, "receiver message");
            match msg {
                JsonRPC::Request(request) => {
                    tracing::warn!(?request, "server received request, this should happen");
                }
                JsonRPC::Response(res) => {
                    tracing::trace!("{res:?}");

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

    pub(crate) async fn download(&self, params: DownloadParams) -> Result<()> {
        let res = self.send_request("$/download", params).await.await?;
        if let Some(error) = res.error {
            Err(eyre!("request error: {:?}", error))
        } else {
            Ok(())
        }
    }
}

/// Try to assign agents a nixos configuration based the store path of the current system
/// (`/run/current-system`).
async fn match_agent_to_configuration(pool: PgPool) -> Result<()> {
    sqlx::query!(
        "UPDATE agents SET nixos_configuration_id = (
            SELECT e.nixos_configuration_id 
                FROM nixos_configuration_evaluations AS e 
            WHERE agents.current_system = e.store_path)
        WHERE agents.nixos_configuration_id IS NULL"
    )
    .execute(&pool)
    .await?;

    Ok(())
}
