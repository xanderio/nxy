#![allow(unused)]

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use chrono::{serde::ts_seconds, DateTime, Utc};
use color_eyre::{
    eyre::{ensure, eyre},
    Result,
};
use futures_util::Stream;
use serde::Deserialize;
use sqlx::PgPool;
use tokio::process::Command;
use tracing::instrument;

#[derive(Debug)]
pub struct InputFlake {
    input_flake_id: i64,
    pub flake_url: String,
    description: Option<String>,
    //TODO: we probably want to create a gcroot for these paths
    path: String,
    revision: String,
    last_modified: DateTime<Utc>,
    url: String,
    locks: serde_json::Value,
}

impl InputFlake {
    pub async fn update(&self) -> Result<()> {
        tracing::info!("Checking for update {}", self.flake_url);
        let meta = flake_metadata(&self.flake_url).await?;
        if meta.revision != self.revision {
            println!("Change detected: {}", self.flake_url);
            println!("{} -> {}", self.revision, meta.revision);
            println!("{} -> {}", self.last_modified, meta.last_modified);
            println!()
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct InputFlakeStore {
    pool: PgPool,
}

impl InputFlakeStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[instrument]
    pub async fn get_by_flake_url(&self, flake_url: String) -> Result<Option<InputFlake>> {
        sqlx::query_as!(
            InputFlake,
            "select * from input_flakes where flake_url = $1",
            flake_url
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Into::into)
    }

    #[instrument]
    pub async fn add(&self, flake_url: String) -> Result<InputFlake> {
        let meta = flake_metadata(&flake_url).await?;
        sqlx::query_as!(
            InputFlake,
            r#"insert into input_flakes 
                (flake_url, description, path, revision, last_modified, url, locks) values 
                ($1, $2, $3, $4, $5, $6, $7)
                returning *"#,
            flake_url,
            meta.description,
            meta.path,
            meta.revision,
            meta.last_modified,
            meta.url,
            meta.locks
        )
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    #[instrument]
    pub async fn get_or_add(&self, flake_url: String) -> Result<InputFlake> {
        match self.get_by_flake_url(flake_url.clone()).await? {
            Some(flake) => Ok(flake),
            None => self.add(flake_url).await,
        }
    }

    pub async fn stream(&self) -> impl Stream<Item = sqlx::Result<InputFlake>> + '_ {
        sqlx::query_as!(InputFlake, "select * from input_flakes").fetch(&self.pool)
    }
}

#[derive(Debug, Deserialize)]
struct FlakeMeta {
    description: Option<String>,
    path: String,
    #[serde(rename = "lastModified", with = "ts_seconds")]
    last_modified: DateTime<Utc>,
    revision: String,
    url: String,
    locks: serde_json::Value,
}

#[instrument]
async fn flake_metadata(flake_url: &str) -> Result<FlakeMeta> {
    let out = Command::new("nix")
        .args(["flake", "metadata", "--json", flake_url])
        .output()
        .await?;

    ensure!(out.status.success(), "nix metadata failed");
    serde_json::from_slice(&out.stdout).map_err(Into::into)
}
