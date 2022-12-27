# Adapted from the Colmena test framework.

{ self, ... }:
{
  perSystem = { pkgs, lib, ... }:
    let
      server = "server"; # Node configurated as nxy server
      clients = [ "alpha" "beta" "gamma" ]; # Nodes configurated as nxy clients

      # Utilities
      nixosLib = import (pkgs.path + "/nixos/lib") { };

      nxyTestModule = { config, lib, ... }:
        let
          cfg = config.nxy.test;

          clientList = "[${lib.concatStringsSep ", " clients}]";
          flake = pkgs.runCommand "${config.name}-flake" { } '' 
            cp -r ${cfg.flake} $out
            chmod u+w $out
          '';
        in
        {
          options.nxy.test = {
            flake = lib.mkOption {
              description = "Path to a directory containing a flake that is added as an nxy input";
              type = lib.types.path;
            };

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
            testScript =
              let
                inherit (self.inputs) nixpkgs;
                nixpkgsPath = "path:${nixpkgs.outPath}?narHash=${nixpkgs.narHash}";
              in
              ''
                clients = ${clientList}

                start_all()

                server.succeed("cp --no-preserve=mode -r ${flake} /tmp/flake && chmod u+w /tmp/flake")

                server.succeed("sed -i 's @nixpkgs@ ${nixpkgsPath} g' /tmp/flake/flake.nix")
                server.succeed("cd /tmp/flake && nix --extra-experimental-features \"nix-command flakes\" flake lock")
              
                with subtest("Create git repository and commit flake"):
                  server.succeed("git config --global user.email nxy@example.com")
                  server.succeed("git config --global user.name nxy")
                  server.succeed("cd /tmp/flake && git init && git add flake.nix flake.lock")
                  server.succeed("cd /tmp/flake && git commit --message 'Initial commit'")

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
      serverConfig = { pkgs, config, ... }: {
        imports = [ self.nixosModules.server ];
        networking.firewall.enable = false;
        environment.systemPackages = [ pkgs.jq pkgs.git pkgs.nxy-cli ];
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
      _module.args = {
        inherit nodes;

        runTests = module: (evalTest ({ config, ... }: {
          imports = [ module { inherit nodes; } ];
          result = config.test;
        })).config.result;
      };
    };
}
