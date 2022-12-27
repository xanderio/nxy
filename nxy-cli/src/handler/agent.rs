use crate::{
    args::{AgentAction, Format},
    utils::{format_output, format_url},
};
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use tabled::Tabled;

pub(crate) fn handle(action: AgentAction, format: Format) -> Result<()> {
    match action {
        AgentAction::List => list_agents(format),
    }
}

#[derive(Debug, Deserialize, Serialize, Tabled)]
struct Agent {
    #[tabled(rename = "Id")]
    id: uuid::Uuid,

    #[tabled(rename = "Current System")]
    current_system: String,
}

fn list_agents(format: Format) -> Result<()> {
    let agents: Vec<Agent> = ureq::get(&format_url("/api/v1/agent"))
        .call()?
        .into_json()?;

    println!("{}", format_output(agents, format));
    Ok(())
}
