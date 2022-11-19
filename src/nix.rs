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

/// returns names of all nixosConfigurations
#[instrument]
pub async fn list_configurations(flake_url: &str) -> Result<Vec<String>> {
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
pub async fn build_configuration(flake_url: &str, name: &str) -> Result<Value> {
    let mut cmd = Command::new("nix");
    cmd.args([
        "build",
        "--json",
        "--no-link",
        format!("{flake_url}#nixosConfigurations.{name}.config.system.build.toplevel").as_str(),
    ]);

    json_output(cmd).await
}

#[instrument(skip(pool))]
pub async fn build_all_configurations(pool: PgPool, flake_revision_id: i64) -> Result<()> {
    let flake_url = sqlx::query_scalar!(
        "SELECT url FROM flake_revisions WHERE flake_revision_id = $1",
        flake_revision_id
    )
    .fetch_one(&pool)
    .await?;

    let configs = list_configurations(flake_url.as_str()).await?;

    for config in configs.into_iter() {
        let flake_url = flake_url.clone();
        tokio::spawn(async move { build_configuration(&flake_url, &config).await });
    }

    Ok(())
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
