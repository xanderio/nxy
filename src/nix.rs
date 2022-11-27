use chrono::{DateTime, Utc};
use color_eyre::{eyre::eyre, Help, Report, Result, SectionExt};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;
use sqlx::PgPool;
use tokio::process::Command;
use tracing::instrument;

#[derive(Debug, Clone, Deserialize)]
pub struct FlakeMetadata {
    pub revision: String,
    #[serde(rename = "lastModified", with = "chrono::serde::ts_seconds")]
    pub last_modified: DateTime<Utc>,
    pub url: String,
}

/// Query flake metadata with `nix flake metadata`
#[instrument(err)]
pub async fn flake_metadata(flake_url: &str) -> Result<(FlakeMetadata, Value)> {
    let mut cmd = Command::new("nix");
    cmd.args(["flake", "metadata", "--json", flake_url]);

    let meta: serde_json::Value = json_output(cmd).await?;
    let metadata: FlakeMetadata = serde_json::from_value(meta.clone())?;

    Ok((metadata, meta))
}

#[instrument(skip_all)]
pub(crate) async fn update_flakes(db: &PgPool) -> Result<()> {
    let flakes = sqlx::query!(
        r#"
        WITH last_rev AS (
            SELECT flake_id, MAX(flake_revision_id) AS flake_revision_id
            FROM flake_revisions
            GROUP BY flake_id
        )
        SELECT flakes.flake_id, flake_url, revision, last_modified 
        FROM flakes
        JOIN last_rev USING (flake_id)
        JOIN flake_revisions USING (flake_revision_id)
        "#
    )
    .fetch_all(db)
    .await?;

    for flake in flakes {
        tracing::info!("updating {}", flake.flake_url);
        let (metadata, meta) = flake_metadata(&flake.flake_url).await?;
        if metadata.revision == flake.revision {
            continue;
        }
        let flake_revision_id = sqlx::query_scalar!(
            r#"
            INSERT INTO flake_revisions (flake_id, revision, last_modified, url, metadata)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING flake_revision_id
            "#,
            flake.flake_id,
            metadata.revision,
            metadata.last_modified,
            metadata.url,
            meta
        )
        .fetch_one(db)
        .await?;

        process_configurations(db.clone(), flake_revision_id).await?;
    }
    Ok(())
}

#[instrument(skip(db))]
pub(crate) async fn process_configurations(db: PgPool, flake_revision_id: i64) -> Result<()> {
    tracing::info!("foo");
    let revision = sqlx::query!(
        "SELECT flake_id, url FROM flake_revisions WHERE flake_revision_id = $1",
        flake_revision_id
    )
    .fetch_one(&db)
    .await?;

    let configs = list_configurations(&revision.url).await?;
    for config in configs {
        let config_id = upsert_nixos_configuration(&db, revision.flake_id, &config).await?;

        let store_path = config_store_path(&revision.url, &config).await?;
        insert_nixos_configutaion_evaluation(&db, flake_revision_id, config_id, &store_path)
            .await?;
    }
    Ok(())
}

#[instrument(skip(db))]
async fn insert_nixos_configutaion_evaluation(
    db: &PgPool,
    flake_revision_id: i64,
    config_id: i64,
    store_path: &str,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO nixos_configuration_evaluations (flake_revision_id, nixos_configuration_id, store_path)
        VALUES ($1, $2, $3) 
        "#,
        flake_revision_id, config_id, store_path
    )
    .execute(db)
    .await?;

    Ok(())
}

#[instrument(skip(db))]
async fn upsert_nixos_configuration(db: &PgPool, flake_id: i64, name: &str) -> sqlx::Result<i64> {
    if let Some(id) = sqlx::query_scalar!(
        r#"
        INSERT INTO nixos_configurations (flake_id, name)
        VALUES ($1, $2) 
        ON CONFLICT DO NOTHING
        RETURNING nixos_configuration_id
        "#,
        flake_id,
        name
    )
    .fetch_optional(db)
    .await?
    {
        Ok(id)
    } else {
        sqlx::query_scalar!(
            r#"
            SELECT nixos_configuration_id FROM nixos_configurations
            WHERE flake_id = $1 AND name = $2
            "#,
            flake_id,
            name
        )
        .fetch_one(db)
        .await
    }
}

/// returns names of all nixosConfigurations
#[instrument]
pub(crate) async fn list_configurations(flake_url: &str) -> Result<Vec<String>> {
    let mut cmd = Command::new("nix");
    cmd.args([
        "eval",
        "--json",
        format!("{}#nixosConfigurations", flake_url).as_str(),
        "--apply",
        "builtins.attrNames",
    ]);

    json_output(cmd).await
}

#[instrument]
pub async fn config_store_path(flake_url: &str, name: &str) -> Result<String> {
    tracing::info!("evaluating configuration");
    let mut cmd = Command::new("nix");
    cmd.args([
        "path-info",
        "--json",
        format!("{flake_url}#nixosConfigurations.{name}.config.system.build.toplevel").as_str(),
    ]);

    #[derive(Deserialize)]
    struct PathInfo {
        path: String,
    }
    let mut res: Vec<PathInfo> = json_output(cmd).await?;
    tracing::info!("done");
    Ok(res.pop().unwrap().path)
}

/// Executes `cmd` and parse stdout as json
#[instrument]
async fn json_output<T: DeserializeOwned>(mut cmd: Command) -> Result<T> {
    let output = cmd.output().await?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(eyre!("cmd exited with non-zero status code")
            .with_section(move || stdout.trim().to_string().header("Stdout:"))
            .with_section(move || stderr.trim().to_string().header("Stderr:")));
    }

    serde_json::from_str(&stdout).map_err(|e| {
        Report::new(e).with_section(move || stdout.trim().to_string().header("Stdout:"))
    })
}
