use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct State {
    pub id: Uuid,
}

impl State {
    pub fn new() -> Self {
        Self { id: Uuid::new_v4() }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

pub fn load(state_path: Option<PathBuf>) -> State {
    let state_path = state_path.unwrap_or_default();
    let state_file = state_path.join("state.json");
    if state_file.is_file() {
        tracing::info!(file = ?state_file, "Loading state");
        let data = std::fs::read_to_string(state_file).expect("unable to read state file");
        serde_json::from_str(&data).expect("failed to deserialize state file")
    } else {
        tracing::info!(file = ?state_file, "Creating new state file");
        std::fs::create_dir_all(state_path).unwrap();
        let state = State::new();
        std::fs::write(state_file, serde_json::to_vec_pretty(&state).unwrap()).unwrap();
        state
    }
}
