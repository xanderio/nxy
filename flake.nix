{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    cargo2nix = {
      url = "github:cargo2nix/cargo2nix/unstable";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, cargo2nix, fenix, utils, ... }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ cargo2nix.overlays.default ];
        };

        rustToolchain = fenix.packages."${system}".stable.toolchain;

        rustPkgs = pkgs.rustBuilder.makePackageSet {
          inherit rustToolchain;
          packageFun = import ./Cargo.nix;
          packageOverrides = pkgs: pkgs.rustBuilder.overrides.all ++ [
            (pkgs.rustBuilder.rustLib.makeOverride {
              name = "tokio";
              overrideAttrs = drv: {
                rustcflags = drv.rustcflags or [ ] ++ [ "--cfg" "tokio_unstable" ];
              };
            })
          ];
        };
      in
      {
        packages = rec {
          nxy-server = rustPkgs.workspace.nxy-server { };
          nxy-agent = rustPkgs.workspace.nxy-agent { };

          default = nxy-server;
        };

        devShells.default =
          let
            xdg_runtime_dir =
              if builtins.getEnv "XDG_RUNTIME_DIR" == "" then
                ".pg"
              else
                builtins.getEnv "XDG_RUNTIME_DIR";
          in
          rustPkgs.workspaceShell {
            RUSTFLAGS = "--cfg tokio_unstable";

            PGDATA = ".pg/data";
            PGHOST = "${xdg_runtime_dir}/nxy";
            PGDATABASE = "nxy";
            DATABASE_URL = "postgres://";

            nativeBuildInputs = with pkgs; [
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
