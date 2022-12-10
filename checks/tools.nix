# Adapted from the Colmena test framework.

{ self
, pkgs
, server ? "server"                  # Node configurated as nxy server
, clients ? [ "alpha" "beta" "gamma" ]   # Nodes configurated as nxy clients
}:
let
  inherit (pkgs) lib;

  # Utilities
  nixosLib = import (pkgs.path + "/nixos/lib") { };

  nxyTestModule = { config, lib, ... }:
    let
      cfg = config.nxy.test;

      clientList = "[${lib.concatStringsSep ", " clients}]";
    in
    {
      options.nxy.test = {
        testScript = lib.mkOption {
          description = ''
            The test script. 

            The nxy test framework will prepend initialization
            statments to the actual test script.
          '';
          type = lib.types.str;
        };
      };

      config = {
        testScript = ''
          clients = ${clientList}

          start_all()

          server.wait_for_unit("nxy-server.service")
          server.wait_for_open_port(8080)
          for node in clients:
            node.wait_for_unit("nxy-agent.service")

          ${cfg.testScript}
        '';
      };
    };

  evalTest = module: nixosLib.evalTest {
    imports = [
      module
      nxyTestModule
      { hostPkgs = pkgs; }
    ];
  };

  ## Common setup 

  # Setup for server node
  serverConfig = { ... }: {
    imports = [ self.nixosModules.server ];
    environment.systemPackages = [ pkgs.jq ];
    services.nxy-server.enable = true;
    virtualisation = {
      # The server needs to be able to write to the store 
      # in order to build new system configurations
      writableStore = true;
    };
  };

  # Setup for client nodes 
  # 
  # Keep as minimal as possible.
  clientConfig = { ... }: {
    imports = [ self.nixosModules.agent ];
    services.nxy-agent = {
      enable = true;
      server = "ws://server:8080";
    };
  };

  nodes =
    let
      serverNode = lib.nameValuePair server serverConfig;
      clientNodes = map (name: lib.nameValuePair name clientConfig) clients;
    in
    lib.listToAttrs ([ serverNode ] ++ clientNodes);
in
{
  inherit pkgs nodes;

  runTests = module: (evalTest ({ config, ... }: {
    imports = [ module { inherit nodes; } ];
    result = config.test;
  })).config.result;
}
