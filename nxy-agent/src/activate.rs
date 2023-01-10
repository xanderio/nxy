use std::{collections::HashMap, path::PathBuf};

use color_eyre::{
    eyre::{bail, ensure},
    Result,
};
use serde::Deserialize;
use tokio::process::Command;

pub(crate) type StorePath = PathBuf;

/// Activate the profile
///
/// # Arguments
///
/// * `profile` - Profile name
/// * `store_path` - Store path to activate
pub(crate) async fn activate(profile: String, store_path: StorePath) -> Result<()> {
    if !is_nixos_system(&store_path).await? {
        bail!("only nixos profiles are currently supported");
    }

    set_profile(&profile, &store_path).await?;
    let ac = get_activation_script(&store_path);

    //TODO: how to protect this from service restart?
    let output = Command::new(ac).arg("switch").output().await?;
    if !output.status.success() {
        tracing::error!(stderr = %String::from_utf8_lossy(&output.stderr), stdout = %String::from_utf8_lossy(&output.stdout), "failed to switch profile");
        bail!("failed to switch profile")
    }
    Ok(())
}

/// Build configuration
///
/// # Arguments
///
/// * `flake_url` - nix compatible flake URL, preferable with ref specified
/// * `name` - attribute of nixosConfigurations to build
///
/// # Returns
///
/// store path of the build configuration
pub(crate) async fn build_system(flake_url: String, name: String) -> Result<StorePath> {
    let mut cmd = Command::new("nix");
    cmd.arg("build");
    cmd.arg("--no-link");
    cmd.arg("--json");
    cmd.arg(format!(
        "{flake_url}#nixosConfigurations.{name}.config.system.build.toplevel"
    ));

    let output = cmd.output().await?;
    ensure!(output.status.success(), "nix build failed");

    #[derive(Deserialize)]
    struct BuildResult {
        outputs: HashMap<String, StorePath>,
    }
    let json: Vec<BuildResult> = serde_json::from_slice(&output.stdout)?;
    Ok(json.first().unwrap().outputs.get("out").unwrap().clone())
}

/// Set `profile` to `store_path`
async fn set_profile(profile: &str, store_path: &StorePath) -> Result<()> {
    let system_profiles_dir = PathBuf::from("/nix/var/nix/profiles");
    let profile_dir = system_profiles_dir.join(profile);

    let mut cmd = Command::new("nix-env");
    cmd.arg("--profile");
    cmd.arg(profile_dir);
    cmd.arg("--set");
    cmd.arg(store_path);

    let output = cmd.output().await?;
    ensure!(output.status.success(), "updating profile failed");

    Ok(())
}

/// Returns `true` if `store_path` points to a NixOS system configuration
async fn is_nixos_system(store_path: &StorePath) -> std::io::Result<bool> {
    store_path.join("nixos-version").try_exists()
}

/// Returns location of NixOS activation script
fn get_activation_script(store_path: &StorePath) -> PathBuf {
    store_path.join("bin/switch-to-configuration")
}
