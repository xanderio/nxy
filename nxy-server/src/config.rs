use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_external_url")]
    pub external_url: String,
}

pub fn load_config(path: Option<String>) -> Config {
    if let Some(path) = path {
        tracing::info!(config_path = %path, "loading config");
        let data = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&data).unwrap()
    } else {
        tracing::info!("no config specified, using defaults");
        serde_json::from_value(json!({})).unwrap()
    }
}

fn default_external_url() -> String {
    String::from("http://localhost:8080")
}
