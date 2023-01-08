{inputs, ...}:{
    perSystem = { inputs', pkgs, system, ... }:
    let
      rustToolchain = inputs'.fenix.packages.stable.toolchain;
      rustPkgs = pkgs.rustBuilder.makePackageSet {
        inherit rustToolchain;
        packageFun = import ../Cargo.nix;
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
      _module.args = { inherit rustToolchain rustPkgs; 
            pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = [ inputs.cargo2nix.overlays.default ];
            };
      };

    };
}
