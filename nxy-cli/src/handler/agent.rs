use crate::{
    args::{AgentAction, Format},
    utils::{format_output, format_url},
};
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use tabled::Tabled;
use uuid::Uuid;

pub(crate) fn handle(action: AgentAction, format: Format) -> Result<()> {
    match action {
        AgentAction::List => list_agents(format),
        AgentAction::SetConfig {
            agent_id,
            config_id,
        } => set_configuration(agent_id, config_id),
        AgentAction::Download {
            agent_id,
            store_path,
        } => download_store_path(agent_id, store_path),
        AgentAction::Activate {
            agent_id,
            store_path,
        } => activate(agent_id, store_path),
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

fn set_configuration(agent_id: Uuid, config_id: i64) -> Result<()> {
    ureq::post(&format_url(&format!("/api/v1/agent/{agent_id}")))
        .send_json(ureq::json!({ "config_id": config_id }))
        .unwrap();
    Ok(())
}

fn download_store_path(agent_id: Uuid, store_path: String) -> Result<()> {
    ureq::post(&format_url(&format!("/api/v1/agent/{agent_id}/download")))
        .send_json(ureq::json!({ "store_path": store_path }))
        .unwrap();
    Ok(())
}

fn activate(agent_id: Uuid, store_path: String) -> Result<()> {
    ureq::post(&format_url(&format!("/api/v1/agent/{agent_id}/activate")))
        .send_json(ureq::json!({ "store_path": store_path }))
        .unwrap();
    Ok(())
}
