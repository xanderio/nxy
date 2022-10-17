# SPDX-FileCopyrightText: 2020 Serokell <https://serokell.io/>
# SPDX-FileCopyrightText: 2020 Andreas Fuchs <asf@boinkor.net>
#
# SPDX-License-Identifier: MPL-2.0

{
  description = "A Simple multi-profile Nix-flake deploy tool.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, ... }:
    {
      overlay = final: prev:
        let
          system = final.stdenv.hostPlatform.system;
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
              final.rustPlatform.buildRustPackage
                (darwinOptions // {
                  inherit pname version;

                  src = ./.;
                  cargoBuildFlags = "-p deploy";
                  # disabled for now as this would add a dependency to `self.inputs.nixpkgs.rev`
                  # which whould case a complet rebuild everytime nixpkgs is changed.
                  doCheck = false;

                  cargoLock.lockFile = ./Cargo.lock;
                }) // { meta.description = "A Simple multi-profile Nix-flake deploy tool"; };

            lib = {
              activate = rec {
                custom =
                  {
                    __functor = customSelf: base: activate:
                      final.buildEnv {
                        name = ("activatable-" + base.name);
                        paths =
                          [
                            base
                            (final.writeTextFile {
                              name = base.name + "-activate-path";
                              text = ''
                                #!${final.runtimeShell}
                                set -euo pipefail

                                if [[ "''${DRY_ACTIVATE:-}" == "1" ]]; then
                                    ${customSelf.dryActivate or "echo ${final.writeScript "activate" activate}"}
                                else
                                    ${activate}
                                fi
                              '';
                              executable = true;
                              destination = "/deploy-rs-activate";
                            })
                            (final.writeTextFile {
                              name = base.name + "-activate-rs";
                              text = ''
                                #!${final.runtimeShell}
                                exec ${self.packages.${system}.default}/bin/activate "$@"
                              '';
                              executable = true;
                              destination = "/activate-rs";
                            })
                          ];
                      };
                  };

                nixos = base:
                  (custom // { dryActivate = "$PROFILE/bin/switch-to-configuration dry-activate"; })
                    base.config.system.build.toplevel ''
                    # work around https://github.com/NixOS/nixpkgs/issues/73404
                    cd /tmp

                    $PROFILE/bin/switch-to-configuration switch

                    # https://github.com/serokell/deploy-rs/issues/31
                    ${with base.config.boot.loader;
                    final.lib.optionalString systemd-boot.enable
                    "sed -i '/^default /d' ${efi.efiSysMountPoint}/loader/loader.conf"}
                  '';

                home-manager = base: custom base.activationPackage "$PROFILE/activate";

                noop = base: custom base ":";
              };

              deployChecks = deploy: builtins.mapAttrs (_: check: check deploy) {
                schema = deploy:
                  let
                    deploy-json = final.writeText "deploy.json" (builtins.toJSON deploy);
                  in
                  final.runCommand "jsonschema-deploy-system" { } ''
                    ${final.python3.pkgs.jsonschema}/bin/jsonschema -i ${deploy-json} ${./crates/deploy/interface.json} && touch $out
                  '';

                activate = deploy:
                  let
                    profiles = builtins.concatLists (final.lib.mapAttrsToList
                      (nodeName: node: final.lib.mapAttrsToList
                        (profileName: profile: [ (toString profile.path) nodeName profileName ])
                        node.profiles)
                      deploy.nodes);
                  in
                  final.runCommand "deploy-rs-check-activate" { } ''
                    for x in ${builtins.concatStringsSep " " (map (p: builtins.concatStringsSep ":" p) profiles)}; do
                      profile_path=$(echo $x | cut -f1 -d:)
                      node_name=$(echo $x | cut -f2 -d:)
                      profile_name=$(echo $x | cut -f3 -d:)

                      if ! [[ -f "$profile_path/deploy-rs-activate" ]]; then
                        echo "#$node_name.$profile_name is missing the deploy-rs-activate activation script" 
                        exit 1
                      fi

                      if ! [[ -f "$profile_path/activate-rs" ]]; then
                        echo "#$node_name.$profile_name is missing the activate-rs activation script" 
                        exit 1
                      fi
                    done

                    touch $out
                  '';
              };
            };
          };
        };
      herculesCI = {
        ciSystems = [ "x86_64-linux" ];
      };
    } //
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; overlays = [ self.overlay ]; };
      in
      {
        packages = rec {
          deploy-rs = pkgs.deploy-rs.deploy-rs;
          default = deploy-rs;
        };

        apps = rec {
          deploy-rs = {
            type = "app";
            program = "${self.packages."${system}".default}/bin/deploy";
          };
          default = deploy-rs;
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
            NIXPKGS_REV = self.inputs.nixpkgs.rev;
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
