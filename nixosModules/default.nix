{ self, ... }: {
  flake.nixosModules = {
    agent = import ./modules/agent.nix self;
    server = import ./modules/server.nix self;
  };
}
