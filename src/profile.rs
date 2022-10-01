use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    process::Command,
};

use color_eyre::{eyre::bail, Result};
use merge::Merge;
use serde::{de::DeserializeOwned, Deserialize};
use tracing::instrument;

use crate::{parse_flake, DeployFlake, ParseFlakeError};

#[derive(Debug, Default)]
pub struct Deploy {
    pub nodes: HashMap<String, Node>,
}

impl Deploy {
    fn from_eval_data(data: NixData) -> Self {
        let nodes = data
            .nodes
            .into_iter()
            .map(|(node_name, node)| {
                let profiles = node
                    .profiles
                    .into_iter()
                    .map(|(profile_name, profile)| {
                        let mut merged = data.generic_settings.clone();
                        merged.merge(node.generic_settings.clone());
                        merged.merge(profile.generic_settings.clone());
                        (
                            profile_name,
                            Profile {
                                path: profile.path,
                                profile_path: profile.profile_path,
                                ssh_user: merged.ssh_user,
                                user: merged.user,
                                ssh_opts: if merged.ssh_opts.is_empty() {
                                    None
                                } else {
                                    Some(merged.ssh_opts.join(" "))
                                },
                                fast_connection: merged.fast_connection,
                                auto_rollback: merged.auto_rollback,
                                confirm_timeout: merged.confirm_timeout,
                                temp_path: merged.temp_path,
                                magic_rollback: merged.magic_rollback,
                                sudo: merged.sudo,
                            },
                        )
                    })
                    .collect();
                (
                    node_name,
                    Node {
                        hostname: node.hostname,
                        profiles,
                        profiles_order: node.profiles_order,
                    },
                )
            })
            .collect();
        Deploy { nodes }
    }
}
macro_rules! writeln_some {
    ($w:expr, $name:literal, $field:expr) => {
        if let Some(field) = $field {
            writeln!($w, "      {}: {}", $name, field)?;
        }
    };
}

impl Display for Deploy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (name, node) in &self.nodes {
            writeln!(f, "node: {name}")?;
            writeln!(f, "hostname: {}", node.hostname)?;
            if !node.profiles_order.is_empty() {
                writeln!(f, "profile_order: {:?}", node.profiles_order)?;
            }
            writeln!(f, "profiles:")?;
            for (name, profile) in &node.profiles {
                writeln!(f, "    - name: {name}")?;
                writeln_some!(f, "profile_path", &profile.profile_path);
                writeln_some!(f, "ssh_user", &profile.ssh_user);
                writeln_some!(f, "user", &profile.user);
                writeln_some!(f, "ssh_opts", &profile.ssh_opts);
                writeln_some!(f, "fast_connection", profile.fast_connection);
                writeln_some!(f, "auto_rollback", profile.auto_rollback);
                writeln_some!(f, "confirm_timeout", profile.confirm_timeout);
                writeln_some!(f, "temp_path", profile.confirm_timeout);
                writeln_some!(f, "magic_rollback", profile.magic_rollback);
                writeln_some!(f, "sudo", &profile.sudo);
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Node {
    pub hostname: String,
    pub profiles: HashMap<String, Profile>,
    pub profiles_order: Vec<String>,
}

#[derive(Debug)]
pub struct Profile {
    #[allow(unused)]
    pub path: Option<String>,
    pub profile_path: Option<String>,
    pub ssh_user: Option<String>,
    pub user: Option<String>,
    pub ssh_opts: Option<String>,
    pub fast_connection: Option<bool>,
    pub auto_rollback: Option<bool>,
    pub confirm_timeout: Option<u16>,
    pub temp_path: Option<String>,
    pub magic_rollback: Option<bool>,
    pub sudo: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Merge)]
struct GenericSettings {
    #[serde(rename(deserialize = "sshUser"))]
    ssh_user: Option<String>,
    user: Option<String>,
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        default,
        rename(deserialize = "sshOpts")
    )]
    #[merge(strategy = merge::vec::append)]
    ssh_opts: Vec<String>,
    #[serde(rename(deserialize = "fastConnection"))]
    fast_connection: Option<bool>,
    #[serde(rename(deserialize = "autoRollback"))]
    auto_rollback: Option<bool>,
    #[serde(rename(deserialize = "confirmTimeout"))]
    confirm_timeout: Option<u16>,
    #[serde(rename(deserialize = "tempPath"))]
    temp_path: Option<String>,
    #[serde(rename(deserialize = "magicRollback"))]
    magic_rollback: Option<bool>,
    sudo: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct NixProfile {
    path: Option<String>,
    #[serde(rename(deserialize = "profilePath"))]
    profile_path: Option<String>,

    #[serde(flatten)]
    generic_settings: GenericSettings,
}

#[derive(Deserialize, Debug, Clone)]
struct NixNode {
    hostname: String,
    profiles: HashMap<String, NixProfile>,
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        default,
        rename(deserialize = "profilesOrder")
    )]
    profiles_order: Vec<String>,

    #[serde(flatten)]
    generic_settings: GenericSettings,
}

#[derive(Deserialize, Debug, Clone)]
struct NixData {
    #[serde(flatten)]
    pub generic_settings: GenericSettings,
    pub nodes: HashMap<String, NixNode>,
}

/// Use in evalutation of nodes and profiles without building the profiles,
/// this is only done on a best effort basis by removing the path attr.
const NIX_NODES_WITHOUT_BUILD: &str = r#"
deploy: deploy // { 
   nodes = let 
        nixpkgs = builtins.getFlake "github:nix/nixpkgs/7e52b35fe98481a279d89f9c145f8076d049d2b9";
    in 
    nixpkgs.lib.attrsets.filterAttrsRecursive (n: v: n != "path") deploy.nodes;
}"#;

#[instrument]
pub fn load_deployment_metadata(flakes: &[&str]) -> Result<Deploy> {
    let deploy_flakes: Vec<DeployFlake> = flakes
        .iter()
        .map(|flake| parse_flake(flake))
        .collect::<Result<Vec<DeployFlake>, ParseFlakeError>>()?;

    let mut deploy = Deploy::default();
    for flake in deploy_flakes
        .into_iter()
        .map(|f| f.repo)
        .collect::<HashSet<_>>()
    {
        // get all profiles _without_ building them
        let nix_data: NixData = nix_eval(&format!("{flake}#deploy"), NIX_NODES_WITHOUT_BUILD)?;
        tracing::trace!(?nix_data);
        deploy
            .nodes
            .extend(Deploy::from_eval_data(nix_data).nodes.into_iter());
    }
    Ok(deploy)
}

#[instrument]
fn nix_eval<T: DeserializeOwned>(flake: &str, apply: &str) -> Result<T> {
    let output = Command::new("nix")
        .args(&["eval", "--json", flake, "--apply", apply])
        .output()?;

    if !output.status.success() {
        tracing::debug!(stderr = ?String::from_utf8(output.stderr));
        bail!("nix eval failed");
    }

    serde_json::from_slice(&output.stdout).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_system() {
        let deploy = load_deployment_metadata(&["./examples/system"]).unwrap();

        let expected = r#"node: example
hostname: localhost
profiles:
    - name: hello
      ssh_user: hello
      user: hello
      ssh_opts: -p 2221
      fast_connection: true
    - name: system
      ssh_user: admin
      user: root
      ssh_opts: -p 2221
      fast_connection: true

"#;

        assert_eq!(deploy.to_string(), expected);
    }

    #[test]
    fn display_simple() {
        let deploy = load_deployment_metadata(&["./examples/simple"]).unwrap();

        let expected = r#"node: example
hostname: localhost
profiles:
    - name: hello
      user: balsoft

"#;

        assert_eq!(deploy.to_string(), expected);
    }
}
