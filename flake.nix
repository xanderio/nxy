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
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = { self, flake-parts, ... }@inputs:
    flake-parts.lib.mkFlake { inherit self; } {
      systems = [ "x86_64-linux" ];
      perSystem = { pkgs, system, inputs', ... }:
        let
          rustToolchain = inputs'.fenix.packages.stable.toolchain;
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
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ inputs.cargo2nix.overlays.default ];
          };

          packages = rec {
            nxy-server = rustPkgs.workspace.nxy-server { };
            nxy-agent = rustPkgs.workspace.nxy-agent { };
            nxy-cli = rustPkgs.workspace.nxy-cli { };

            default = nxy-server;
          };

          checks = import ./checks { inherit self pkgs; };

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
        };

      flake = {
        overlays.default = final: prev:
          let
            inherit (final.stdenv) system;
          in
          {
            inherit (self.packages.${system}) nxy-agent nxy-server nxy-cli;
          };
        nixosModules = {
          agent = import ./modules/agent.nix self;
          server = import ./modules/server.nix self;
        };
      };
    };
}
