use clap::{Parser, Subcommand};

#[derive(Parser)]
pub(crate) struct Args {
    #[command(subcommand)]
    pub(crate) action: Action,
}

#[derive(Subcommand)]
pub(crate) enum Action {
    /// interact with nxy agents
    Agents {
        #[command(subcommand)]
        action: AgentAction,
    },
}

#[derive(Subcommand)]
pub(crate) enum AgentAction {
    /// List all agents
    List,
}
