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
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" ];
      imports = [
        inputs.proc-flake.flakeModule
        inputs.flake-root.flakeModule
        ./modules
        ./checks
        ./nixosModules
      ];

      flake = {
        overlays.default = final: prev:
          let
            inherit (final.stdenv) system;
          in
          {
            inherit (self.packages.${system}) nxy-agent nxy-server nxy-cli;
          };
      };
    };
}
