# SPDX-FileCopyrightText: 2020 Serokell <https://serokell.io/>
# SPDX-FileCopyrightText: 2020 Andreas Fuchs <asf@boinkor.net>
#
# SPDX-License-Identifier: MPL-2.0

{
  description = "A Simple multi-profile Nix-flake deploy tool.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    naersk.url = "github:nix-community/naersk";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, naersk, utils, ... }:
    {
      overlays.default = final: prev:
        let
          system = final.stdenv.hostPlatform.system;
          naersk' = final.callPackage naersk { };
          darwinOptions = final.lib.optionalAttrs final.stdenv.isDarwin {
            buildInputs = with final.darwin.apple_sdk.frameworks; [
              SystemConfiguration
              CoreServices
            ];
          };
        in
        {
          deploy-rs = {
            deploy-rs =
              let
                cargoToml = builtins.fromTOML (builtins.readFile ./crates/deploy/Cargo.toml);
                pname = cargoToml.package.name;
                version = cargoToml.package.version;
              in
              naersk'.buildPackage
                (darwinOptions // {
                  inherit pname version;

                  src = ./.;
                  cargoBuildOptions = x: x ++ [ "--package deploy" ];

                }) // { meta.description = "A Simple multi-profile Nix-flake deploy tool"; };

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
        naersk' = pkgs.callPackage naersk { };
      in
      {
        packages = rec {
          deploy-rs = pkgs.deploy-rs.deploy-rs;
          nxy =
            let
              cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
              pname = cargoToml.package.name;
              version = cargoToml.package.version;
            in
            naersk'.buildPackage {
              inherit pname version;
              src = ./.;

              RUSTFLAGS = "--cfg tokio_unstable";
            };
          nxy-agent =
            let
              cargoToml = builtins.fromTOML (builtins.readFile ./crates/agent/Cargo.toml);
              pname = cargoToml.package.name;
              version = cargoToml.package.version;
            in
            naersk'.buildPackage {
              inherit pname version;
              src = ./.;
              cargoBuildOptions = x: x ++ [ "--package agent" ];

              RUSTFLAGS = "--cfg tokio_unstable";
            };
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
