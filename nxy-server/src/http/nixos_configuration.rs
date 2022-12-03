use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

use crate::http::Result;

use super::ApiContext;

pub(crate) fn router() -> Router<ApiContext> {
    Router::new().route("/api/v1/configuration", get(list_configurations))
}

#[derive(Debug, Serialize)]
struct Configuration {
    id: i64,
    name: String,
    flake_id: i64,
    flake_url: String,
}

async fn list_configurations(ctx: State<ApiContext>) -> Result<Json<Vec<Configuration>>> {
    let configs = sqlx::query!(
        "SELECT flake_id, flake_url, nixos_configuration_id, name
         FROM nixos_configurations 
         JOIN flakes USING (flake_id)"
    )
    .fetch_all(&ctx.db)
    .await?
    .into_iter()
    .map(|row| Configuration {
        id: row.nixos_configuration_id,
        name: row.name,
        flake_id: row.flake_id,
        flake_url: row.flake_url,
    })
    .collect();

    Ok(Json(configs))
}
