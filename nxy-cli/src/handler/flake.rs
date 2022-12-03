use std::fmt::Display;

use color_eyre::Result;
use serde::Deserialize;
use tabled::{Style, Table, Tabled};

use crate::args::FlakeAction;

pub(crate) fn handle(action: FlakeAction) -> Result<()> {
    match action {
        FlakeAction::List => list_flakes(),
        FlakeAction::Add { flake_url } => add_flake(flake_url),
    }
}

#[derive(Debug, Deserialize, Tabled)]
struct Flake {
    #[tabled(rename = "id")]
    flake_id: i64,
    #[tabled(rename = "url")]
    flake_url: String,
    #[tabled(rename = "current revision")]
    lastest_revision: FlakeRevision,
}

#[derive(Debug, Deserialize)]
struct FlakeRevision {
    revision: String,
}

impl Display for FlakeRevision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.revision)
    }
}

fn list_flakes() -> Result<()> {
    let flakes: Vec<Flake> = ureq::get("http://localhost:8080/api/v1/flake")
        .call()?
        .into_json()?;

    let table = Table::new(flakes).with(Style::rounded()).to_string();
    println!("{table}");

    Ok(())
}

fn add_flake(flake_url: String) -> Result<()> {
    ureq::post("http://localhost:8080/api/v1/flake").send_json(ureq::json!({
        "flake": {
            "flake_url": flake_url
        }
    }))?;
    Ok(())
}
