{ inputs, ... }: {
  perSystem = { inputs', pkgs, system, ... }:
    let
      rustToolchain = inputs'.fenix.packages.stable;
      craneLib = inputs.crane.lib.${system}.overrideToolchain rustToolchain.toolchain;
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      commonArgs = {
        src = inputs.nix-filter.lib.filter {
          root = ./..;
          include = [
            "nxy-common"
            "nxy-cli"
            "nxy-agent"
            "nxy-server"
            "Cargo.toml"
            "Cargo.lock"
          ];
        };

        # enable unstable tokio `tracing` feature
        RUSTFLAGS = "--cfg tokio_unstable";
      };
    in
    {
      _module.args = {
        inherit rustToolchain craneLib cargoArtifacts commonArgs;
      };
    };
}
