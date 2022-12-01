mod args;

use std::fmt::Display;

use args::*;

use clap::Parser;
use color_eyre::Result;
use serde::Deserialize;
use tabled::{Style, Table, Tabled};

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();

    match args.action {
        Action::Agents { action } => handle_agent(action),
        Action::Flakes { action } => handle_flake(action),
    }
}

fn handle_flake(action: FlakeAction) -> Result<()> {
    match action {
        FlakeAction::List => list_flakes(),
        FlakeAction::Add { flake_url } => add_flake(flake_url),
    }
}

fn handle_agent(action: AgentAction) -> Result<()> {
    match action {
        AgentAction::List => list_agents(),
    }
}

fn list_agents() -> Result<()> {
    let table = Table::new(agents()?).with(Style::rounded()).to_string();
    println!("{table}");
    Ok(())
}

#[derive(Debug, Deserialize, Tabled)]
struct Agent {
    #[tabled(rename = "Id")]
    id: uuid::Uuid,

    #[tabled(rename = "Current System")]
    current_system: String,
}

fn agents() -> Result<Vec<Agent>> {
    ureq::get("http://localhost:8080/api/v1/agent")
        .call()?
        .into_json()
        .map_err(Into::into)
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
