mod websocket;

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ApiContext, Result};

pub(crate) fn router() -> Router<ApiContext> {
    Router::new()
        .route("/api/v1/agent", get(get_agents))
        .route("/api/v1/agent/ws", get(websocket::ws_handler))
        .route("/api/v1/agent/:agent_id", post(set_configuration))
        .route(
            "/api/v1/agent/:agent_id/download",
            post(download_store_path),
        )
}

#[derive(Serialize)]
struct Agent {
    id: Uuid,
    current_system: Option<String>,
}

async fn get_agents(ctx: State<ApiContext>) -> Result<Json<Vec<Agent>>> {
    let agents = sqlx::query!("SELECT agent_id, current_system FROM agents")
        .fetch_all(&ctx.db)
        .await?
        .into_iter()
        .map(|row| Agent {
            id: row.agent_id,
            current_system: row.current_system,
        })
        .collect();

    Ok(Json(agents))
}

#[derive(Deserialize)]
struct SetConfiguration {
    config_id: i64,
}

async fn set_configuration(
    ctx: State<ApiContext>,
    Path(agent): Path<Uuid>,
    Json(req): Json<SetConfiguration>,
) -> Result<()> {
    sqlx::query!(
        "UPDATE agents SET nixos_configuration_id = $1 WHERE agent_id = $2",
        req.config_id,
        agent
    )
    .execute(&ctx.db)
    .await?;
    Ok(())
}

#[derive(Deserialize)]
struct DownloadStorePath {
    store_path: String,
}

async fn download_store_path(
    ctx: State<ApiContext>,
    Path(agent_id): Path<Uuid>,
    Json(req): Json<DownloadStorePath>,
) -> Result<()> {
    let agent = ctx.agent_manager.get(agent_id).unwrap();

    agent
        .download(nxy_common::types::DownloadParams {
            store_path: req.store_path.into(),
        })
        .await
        .map_err(Into::into)
}
