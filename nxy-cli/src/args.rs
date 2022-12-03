use clap::{Parser, Subcommand};

#[derive(Parser)]
pub(crate) struct Args {
    #[command(subcommand)]
    pub(crate) action: Action,
}

#[derive(Subcommand)]
pub(crate) enum Action {
    /// interact with nxy flakes
    Flakes {
        #[command(subcommand)]
        action: FlakeAction,
    },
    /// interact with nxy agents
    Agents {
        #[command(subcommand)]
        action: AgentAction,
    },
    /// interact with nixos configurations
    Configs {
        #[command(subcommand)]
        action: ConfigsAction,
    },
}

#[derive(Subcommand)]
pub(crate) enum AgentAction {
    /// List all agents
    List,
}

#[derive(Subcommand)]
pub(crate) enum FlakeAction {
    /// List all flakes
    List,
    Add {
        /// flake uri to add to nxy
        flake_url: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum ConfigsAction {
    /// List all configs
    List,
}
