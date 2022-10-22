# SPDX-FileCopyrightText: 2020 Serokell <https://serokell.io/>
# SPDX-FileCopyrightText: 2020 Andreas Fuchs <asf@boinkor.net>
#
# SPDX-License-Identifier: MPL-2.0

{
  description = "A Simple multi-profile Nix-flake deploy tool.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "utils";
    };
    utils.url = "github:numtide/flake-utils";
    gitignore = {
      url = "github:hercules-ci/gitignore.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, crane, gitignore, utils, ... }:
    {
      overlays.default = final: prev:
        let
          system = final.stdenv.hostPlatform.system;
        in
        {
          deploy-rs = {
            deploy-rs = self.packages.${system}.deploy-rs;

            lib = import ./lib { inherit self final system; };
          };
        };
      herculesCI = {
        ciSystems = [ "x86_64-linux" ];
      };
    } //
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; overlays = [ self.overlays.default ]; };
        inherit (gitignore.lib) gitignoreSource;

        craneLib = crane.lib.${system};
        commonArgs = {
          src = gitignoreSource ./.;

          # enable unstable tokio `tracing` feature
          RUSTFLAGS = "--cfg tokio_unstable";
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      in
      {
        packages = rec {
          nxy = craneLib.buildPackage (commonArgs // {
            inherit cargoArtifacts;
          });

          nxy-agent = craneLib.buildPackage (commonArgs // {
            inherit cargoArtifacts;
            cargoEtraArgs = "--package agent";
          } // craneLib.crateNameFromCargoToml { cargoToml = ./crates/agent/Cargo.toml; });

          deploy-rs = craneLib.buildPackage (commonArgs // {
            inherit cargoArtifacts;
            cargoEtraArgs = "--package deploy";
          } // craneLib.crateNameFromCargoToml { cargoToml = ./crates/deploy/Cargo.toml; });

          default = nxy;
        };

        apps = rec {
          deploy-rs = {
            type = "app";
            program = "${self.packages."${system}".deploy-rs}/bin/deploy";
          };
          nxy = {
            type = "app";
            program = "${self.packages."${system}".nxy}/bin/nxy";
          };
          nxy-agent = {
            type = "app";
            program = "${self.packages."${system}".nxy-agent}/bin/agent";
          };
          default = nxy;
        };

        devShells.default =
          let
            xdg_runtime_dir =
              if builtins.getEnv "XDG_RUNTIME_DIR" == "" then
                ".pg"
              else
                builtins.getEnv "XDG_RUNTIME_DIR";
          in
          pkgs.mkShell {
            inputsFrom = [ self.packages.${system}.deploy-rs ];
            RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
            RUSTFLAGS = "--cfg tokio_unstable";
            PGDATA = ".pg/data";
            PGHOST = "${xdg_runtime_dir}/nxy";
            PGDATABASE = "nxy";
            DATABASE_URL = "postgres://";
            buildInputs = with pkgs; [
              nixUnstable
              cargo
              rustc
              rustfmt
              clippy
              reuse
              rust.packages.stable.rustPlatform.rustLibSrc
              postgresql_14
              sqlx-cli
              websocat
            ];
            shellHook = ''
              mkdir -p $XDG_RUNTIME_DIR/nxy
              if ! [ -d $PGDATA ]; then 
                initdb
              fi
              if ! pg_ctl status > /dev/null; then
                systemd-run --user --unit=nxy_postgres --service-type=notify \
                  --same-dir -E PGDATA=$PGDATA \
                  ${pkgs.postgresql_14}/bin/postgres --listen-addresses="" --unix_socket_directories=${xdg_runtime_dir}/nxy

                  createdb
              fi
            '';
          };

        checks = {
          deploy-rs = self.packages.${system}.default.overrideAttrs (super: { doCheck = true; });
        };

        lib = pkgs.deploy-rs.lib;
      });
}
