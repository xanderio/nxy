use color_eyre::Result;
use serde::Deserialize;
use tabled::{Style, Table, Tabled};

use crate::args::ConfigsAction;

pub(crate) fn handle(action: ConfigsAction) -> Result<()> {
    match action {
        ConfigsAction::List => list_configs(),
    }
}

#[derive(Deserialize, Tabled)]
struct Config {
    #[tabled(rename = "flake url")]
    flake_url: String,
    name: String,
}

fn list_configs() -> Result<()> {
    let configs: Vec<Config> = ureq::get("http://localhost:8080/api/v1/configuration")
        .call()?
        .into_json()?;

    let table = Table::new(configs).with(Style::rounded()).to_string();
    println!("{table}");

    Ok(())
}
