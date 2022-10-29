use axum::{routing::get, Extension, Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{http::Result, nix::flake_metadata};

pub fn router() -> Router {
    Router::new().route(
        "/v1/flake",
        get(get_flakes).post(create_flake).put(update_flake),
    )
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
struct CreateFlakeRequest {
    flake_url: String,
}

async fn create_flake(
    Extension(db): Extension<PgPool>,
    Json(req): Json<CreateFlakeRequest>,
) -> Result<Json<Flake>> {
    // fetch flake metadata, this also validates the flake url
    let (metadata, meta) = flake_metadata(&req.flake_url).await?;

    let result = sqlx::query!(
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
        req.flake_url,
        metadata.revision,
        metadata.last_modified,
        metadata.url,
        meta
    )
    .fetch_one(&db)
    .await?;

    let revision = FlakeRevision {
        flake_revision_id: result.flake_revision_id,
        revision: result.revision,
        last_modified: result.last_modified.to_string(),
        url: result.url,
    };

    let flake = Flake {
        flake_id: result.flake_id,
        flake_url: result.flake_url,
        lastest_revision: revision,
    };

    Ok(Json(flake))
}

async fn get_flakes(Extension(db): Extension<PgPool>) -> Result<Json<Vec<Flake>>> {
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
    .fetch_all(&db).await?
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

async fn update_flake(Extension(db): Extension<PgPool>) -> Result<()> {
    let flakes = sqlx::query!(
        r#"
        with last_rev as (
            select flake_id, max(flake_revision_id) as flake_revision_id
            from flake_revisions
            group by flake_id
        )
        select flakes.flake_id, flake_url, revision, last_modified 
        from flakes
        join last_rev using (flake_id)
        join flake_revisions using (flake_revision_id)
        "#
    )
    .fetch_all(&db)
    .await?;

    for flake in flakes {
        tracing::info!("updating {}", flake.flake_url);
        let (metadata, meta) = flake_metadata(&flake.flake_url).await?;
        if metadata.revision == flake.revision {
            continue;
        }
        sqlx::query!(
            r#"
            insert into flake_revisions (flake_id, revision, last_modified, url, metadata)
            values ($1, $2, $3, $4, $5)
            "#,
            flake.flake_id,
            metadata.revision,
            metadata.last_modified,
            metadata.url,
            meta
        )
        .execute(&db)
        .await?;
    }
    Ok(())
}
