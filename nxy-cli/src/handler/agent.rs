use crate::{args::AgentAction, utils::format_url};
use color_eyre::Result;
use serde::Deserialize;
use tabled::{Style, Table, Tabled};

pub(crate) fn handle(action: AgentAction) -> Result<()> {
    match action {
        AgentAction::List => list_agents(),
    }
}

#[derive(Debug, Deserialize, Tabled)]
struct Agent {
    #[tabled(rename = "Id")]
    id: uuid::Uuid,

    #[tabled(rename = "Current System")]
    current_system: String,
}

fn list_agents() -> Result<()> {
    let agents: Vec<Agent> = ureq::get(&format_url("/api/v1/agent"))
        .call()?
        .into_json()?;

    let table = Table::new(agents).with(Style::rounded()).to_string();
    println!("{table}");
    Ok(())
}
