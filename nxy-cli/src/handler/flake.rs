use std::fmt::Display;

use color_eyre::Result;
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use crate::{
    args::{FlakeAction, Format},
    utils::{format_output, format_url},
};

pub(crate) fn handle(action: FlakeAction, format: Format) -> Result<()> {
    match action {
        FlakeAction::List => list_flakes(format),
        FlakeAction::Add { flake_url } => add_flake(flake_url),
    }
}

#[derive(Debug, Deserialize, Serialize, Tabled)]
struct Flake {
    #[tabled(rename = "id")]
    flake_id: i64,
    #[tabled(rename = "url")]
    flake_url: String,
    #[tabled(rename = "current revision")]
    lastest_revision: FlakeRevision,
}

#[derive(Debug, Deserialize, Serialize)]
struct FlakeRevision {
    revision: String,
}

impl Display for FlakeRevision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.revision)
    }
}

fn list_flakes(format: Format) -> Result<()> {
    let flakes: Vec<Flake> = ureq::get(&format_url("/api/v1/flake"))
        .call()?
        .into_json()?;

    println!("{}", format_output(flakes, format));

    Ok(())
}

fn add_flake(flake_url: String) -> Result<()> {
    ureq::post(&format_url("/api/v1/flake")).send_json(ureq::json!({
        "flake": {
            "flake_url": flake_url
        }
    }))?;
    Ok(())
}
