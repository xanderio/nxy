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
          server.wait_for_open_port(8085)
          server.wait_for_unit("nix-serve.service")
          server.wait_for_unit("nginx.service")
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
  serverConfig = { config, ... }: {
    imports = [ self.nixosModules.server ];
    networking.firewall.enable = false;
    environment.systemPackages = [ pkgs.jq ];
    services.nxy-server.enable = true;
    services.nix-serve.enable = true;
    services.nginx = {
      enable = true;
      upstreams."nix-serve".servers."localhost:${toString config.services.nix-serve.port}" = { };
      virtualHosts."nxy" = {
        default = true;
        locations."/" = {
          proxyPass = "http://127.0.0.1:8085";
          proxyWebsockets = true;
        };

        locations."/nix-cache-info".proxyPass = "http://nix-serve";
        locations."/nar".proxyPass = "http://nix-serve";
        locations."~ /*.\\.narinfo".proxyPass = "http://nix-serve";
      };
    };
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
      server = "ws://server:80";
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
