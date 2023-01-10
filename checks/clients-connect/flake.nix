{
  description = "A simple deployment";

  inputs.nixpkgs.url = "@nixpkgs@";
  inputs.nxy.url = "@nxy@";

  outputs = { self, nixpkgs, nxy }:
    let
      inherit (nixpkgs) lib;
      system = "x86_64-linux";

      mkSystem = name: lib.nixosSystem {
        inherit system;
        modules = [
        (nixpkgs + "/nixos/lib/testing/nixos-test-base.nix")
        (nxy.checks.x86_64-linux.clients-connect.nodes.${name}.system.build.networkConfig)
        {
          imports = [ nxy.nixosModules.agent ];
          services.nxy-agent = {
            enable = true;
            server = "ws://server:80";
          };
          boot.loader.grub.enable = false;
          networking.hostName = name;
        }];
      };
    in
    {
      nixosConfigurations = lib.genAttrs [ "alpha" ] mkSystem;
    };
}
