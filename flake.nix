{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "utils";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    utils.url = "github:numtide/flake-utils";
    gitignore = {
      url = "github:hercules-ci/gitignore.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, crane, fenix, gitignore, utils, ... }:
    {
      herculesCI = {
        ciSystems = [ "x86_64-linux" ];
      };
    } //
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        inherit (gitignore.lib) gitignoreSource;

        rustToolchain = fenix.packages.${system}.stable;

        craneLib = crane.lib.${system}.overrideToolchain rustToolchain.toolchain;

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
            RUST_SRC_PATH = "${rustToolchain.rust-src}/lib/rustlib/src/rust/library";
            RUSTFLAGS = "--cfg tokio_unstable";
            PGDATA = ".pg/data";
            PGHOST = "${xdg_runtime_dir}/nxy";
            PGDATABASE = "nxy";
            DATABASE_URL = "postgres://";
            inputsFrom = [ self.packages.${system}.nxy ];
            buildInputs = with pkgs; [
              nixUnstable

              # runtime deps
              postgresql_14

              # developer tooling
              python3Packages.pgcli
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
      });
}
