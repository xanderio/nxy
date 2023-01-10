use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub id: Uuid,
    pub system: System,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct System {
    pub current: PathBuf,
    pub booted: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadParams {
    pub store_path: PathBuf,
    pub from: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivateParams {
    pub store_path: PathBuf,
}
