{
  perSystem = { rustPkgs, ... }:
    {
      packages = rec {
        nxy-server = rustPkgs.workspace.nxy-server { };
        nxy-agent = rustPkgs.workspace.nxy-agent { };
        nxy-cli = rustPkgs.workspace.nxy-cli { };

        default = nxy-server;
      };
    };
}
