mod websocket;

use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use uuid::Uuid;

use super::{ApiContext, Result};

pub(crate) fn router() -> Router<ApiContext> {
    Router::new()
        .route("/api/v1/agent", get(get_agents))
        .route("/api/v1/agent/ws", get(websocket::ws_handler))
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
