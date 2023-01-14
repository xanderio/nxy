{ inputs, ... }: {
  perSystem = { inputs', pkgs, system, ... }:
    let
      rustToolchain = inputs'.fenix.packages.stable;
      craneLib = inputs.crane.lib.${system}.overrideToolchain rustToolchain.toolchain;
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      commonArgs = {
        src = inputs.gitignore.lib.gitignoreSource ./..;

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
