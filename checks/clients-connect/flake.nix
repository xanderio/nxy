{
  description = "A simple deployment";

  inputs.nixpkgs.url = "@nixpkgs@";

  outputs = { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;
      system = "x86_64-linux";

      mkSystem = name: lib.nixosSystem {
        inherit system;
        modules = [
        (nixpkgs + "/nixos/lib/testing/nixos-test-base.nix")
        {
          boot.loader.grub.enable = false;
          networking.hostName = name;
        }];
      };
    in
    {
      nixosConfigurations = lib.genAttrs [ "alpha" ] mkSystem;
    };
}
