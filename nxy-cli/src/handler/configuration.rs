use color_eyre::Result;
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use crate::{
    args::{ConfigsAction, Format},
    utils::{format_output, format_url},
};

pub(crate) fn handle(action: ConfigsAction, format: Format) -> Result<()> {
    match action {
        ConfigsAction::List => list_configs(format),
    }
}

#[derive(Deserialize, Serialize, Tabled)]
struct Config {
    id: i64,
    #[tabled(rename = "flake url")]
    flake_url: String,
    name: String,
}

fn list_configs(format: Format) -> Result<()> {
    let configs: Vec<Config> = ureq::get(&format_url("/api/v1/configuration"))
        .call()?
        .into_json()?;

    println!("{}", format_output(configs, format));

    Ok(())
}
