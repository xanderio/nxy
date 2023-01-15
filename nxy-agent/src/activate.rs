use std::{path::PathBuf, process::Command};

use eyre::{bail, ensure, Result};

pub(crate) type StorePath = PathBuf;

/// Activate the profile
///
/// # Arguments
///
/// * `profile` - Profile name
/// * `store_path` - Store path to activate
pub(crate) fn activate(profile: String, store_path: StorePath) -> Result<()> {
    if !is_nixos_system(&store_path)? {
        bail!("only nixos profiles are currently supported");
    }

    set_profile(&profile, &store_path)?;
    let ac = get_activation_script(&store_path);

    //TODO: how to protect this from service restart?
    let output = Command::new(ac).arg("switch").output()?;
    if !output.status.success() {
        tracing::error!(stderr = %String::from_utf8_lossy(&output.stderr), stdout = %String::from_utf8_lossy(&output.stdout), "failed to switch profile");
        bail!("failed to switch profile")
    }
    Ok(())
}

/// Set `profile` to `store_path`
fn set_profile(profile: &str, store_path: &StorePath) -> Result<()> {
    let system_profiles_dir = PathBuf::from("/nix/var/nix/profiles");
    let profile_dir = system_profiles_dir.join(profile);

    let mut cmd = Command::new("nix-env");
    cmd.arg("--profile");
    cmd.arg(profile_dir);
    cmd.arg("--set");
    cmd.arg(store_path);

    let output = cmd.output()?;
    ensure!(output.status.success(), "updating profile failed");

    Ok(())
}

/// Returns `true` if `store_path` points to a NixOS system configuration
fn is_nixos_system(store_path: &StorePath) -> std::io::Result<bool> {
    store_path.join("nixos-version").try_exists()
}

/// Returns location of NixOS activation script
fn get_activation_script(store_path: &StorePath) -> PathBuf {
    store_path.join("bin/switch-to-configuration")
}
