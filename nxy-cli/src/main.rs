mod args;

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
