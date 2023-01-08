{ self, ... }: {
  flake.nixosModules = {
    agent = import ./agent.nix self;
    server = import ./server.nix self;
  };
}
