use chrono::{DateTime, Utc};
use color_eyre::Result;
use serde::Deserialize;
use serde_json::Value;
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
    let output = Command::new("nix")
        .args(["flake", "metadata", "--json", flake_url])
        .output()
        .await?;

    let meta: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let metadata: FlakeMetadata = serde_json::from_slice(&output.stdout)?;

    Ok((metadata, meta))
}
