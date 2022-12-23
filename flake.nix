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
    proc-flake.url = "github:srid/proc-flake";
    flake-root.url = "github:srid/flake-root";
  };

  outputs = { self, flake-parts, ... }@inputs:
    flake-parts.lib.mkFlake { inherit self; } {
      systems = [ "x86_64-linux" ];
      imports = [
        inputs.proc-flake.flakeModule
        inputs.flake-root.flakeModule
        ./modules/services.nix
        ./modules/devshell.nix
      ];
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
          _module.args = {
            inherit rustPkgs;
            pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = [ inputs.cargo2nix.overlays.default ];
            };
          };

          packages = rec {
            nxy-server = rustPkgs.workspace.nxy-server { };
            nxy-agent = rustPkgs.workspace.nxy-agent { };
            nxy-cli = rustPkgs.workspace.nxy-cli { };

            default = nxy-server;
          };

          checks = import ./checks { inherit self pkgs; };

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
