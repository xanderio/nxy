#![allow(unused)]

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use color_eyre::{eyre::eyre, Result};
use serde::Deserialize;
use sqlx::PgPool;

pub struct InputFlake {
    flake_url: String,
}

pub struct InputFlakeStore {
    pool: PgPool,
}

impl InputFlakeStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_or_add(&self, flake_url: String) -> Result<InputFlake> {
        let id = sqlx::query_scalar!(
            "select input_flake_id from input_flakes where flake_url = $1",
            flake_url
        )
        .fetch_optional(&self.pool)
        .await?;
        if id.is_none() {
            sqlx::query_scalar!(
                "insert into input_flakes (flake_url) values ($1)",
                flake_url
            )
            .fetch_one(&self.pool)
            .await?;
        }
        Ok(InputFlake { flake_url })
    }
}
