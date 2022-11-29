use axum::{extract::State, routing::get, Json, Router};
use serde::{Deserialize, Serialize};

use crate::nix::{self, flake_metadata, process_configurations};

use super::{ApiContext, Result};

pub(crate) fn router() -> Router<ApiContext> {
    Router::new().route(
        "/api/v1/flake",
        get(get_flakes).post(create_flake).put(update_flake),
    )
}

#[derive(Serialize, Deserialize)]
struct FlakeBody<T> {
    flake: T,
}

#[derive(Serialize)]
struct Flake {
    flake_id: i64,
    flake_url: String,
    lastest_revision: FlakeRevision,
}

#[derive(Serialize)]
struct FlakeRevision {
    flake_revision_id: i64,
    revision: String,
    last_modified: String,
    url: String,
}

#[derive(Deserialize)]
struct NewFlake {
    flake_url: String,
}

async fn create_flake(
    ctx: State<ApiContext>,
    Json(req): Json<FlakeBody<NewFlake>>,
) -> Result<Json<FlakeBody<Flake>>> {
    // fetch flake metadata, this also validates the flake url
    let (metadata, meta) = flake_metadata(&req.flake.flake_url).await?;

    let flake = sqlx::query!(
        r#"
            with inserted_flake as (
                insert into flakes (flake_url)
                values ($1)
                returning flake_id, flake_url
            ), inserted_revision as (
                insert into flake_revisions (flake_id, revision, last_modified, url, metadata)
                select flake_id, $2, $3, $4, $5
                from inserted_flake
                returning flake_revision_id, revision, last_modified, url
            )
            select flake_id, flake_url, flake_revision_id, revision, last_modified, url
            from inserted_flake, inserted_revision
        "#,
        req.flake.flake_url,
        metadata.revision,
        metadata.last_modified,
        metadata.url,
        meta
    )
    .fetch_one(&ctx.db)
    .await?;

    tokio::spawn(process_configurations(
        ctx.db.clone(),
        flake.flake_revision_id,
    ));

    Ok(Json(FlakeBody {
        flake: Flake {
            flake_id: flake.flake_id,
            flake_url: flake.flake_url,
            lastest_revision: FlakeRevision {
                flake_revision_id: flake.flake_revision_id,
                revision: flake.revision,
                last_modified: flake.last_modified.to_string(),
                url: flake.url,
            },
        },
    }))
}

async fn get_flakes(ctx: State<ApiContext>) -> Result<Json<Vec<Flake>>> {
    let flakes = sqlx::query!(
        r#"
        with last_rev as (
            select flake_id, max(flake_revision_id) as flake_revision_id
            from flake_revisions
            group by flake_id
        )
        select flakes.flake_id, flake_url, flake_revision_id as "flake_revision_id!", revision, last_modified, url
        from flakes
        join last_rev using (flake_id)
        join flake_revisions using (flake_revision_id)
        "#,
    )
    .fetch_all(&ctx.db).await?
    .into_iter()
    .map(|row| {
        let revision = FlakeRevision {
            flake_revision_id: row.flake_revision_id,
            revision: row.revision,
            last_modified: row.last_modified.to_string(),
            url: row.url,
        };

        Flake {
            flake_id: row.flake_id,
            flake_url: row.flake_url,
            lastest_revision: revision,
        }
    })
    .collect();

    Ok(Json(flakes))
}

async fn update_flake(ctx: State<ApiContext>) -> Result<()> {
    nix::update_flakes(&ctx.db).await?;
    Ok(())
}
