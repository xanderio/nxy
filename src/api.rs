use axum::{routing::get, Extension, Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::http::Result;

pub fn router() -> Router {
    Router::new().route("/v1/flake", get(get_flakes).post(create_flake))
}

#[derive(Serialize)]
struct Flake {
    flake_id: i64,
    flake_url: String,
}

#[derive(Deserialize)]
struct CreateFlakeRequest {
    flake_url: String,
}

async fn create_flake(
    Extension(db): Extension<PgPool>,
    Json(req): Json<CreateFlakeRequest>,
) -> Result<Json<Flake>> {
    let flake = sqlx::query_as!(
        Flake,
        r#"
            insert into flakes (flake_url)
            values ($1)
            returning flake_id, flake_url
        "#,
        req.flake_url
    )
    .fetch_one(&db)
    .await?;

    Ok(Json(flake))
}

async fn get_flakes(Extension(db): Extension<PgPool>) -> Result<Json<Vec<Flake>>> {
    let flakes = sqlx::query_as!(
        Flake,
        r#"
            select flake_id, flake_url 
            from flakes
            order by flake_id
        "#,
    )
    .fetch_all(&db)
    .await?;

    Ok(Json(flakes))
}
