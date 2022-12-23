{ ... }: {
  perSystem = { lib, pkgs, runtimeDir, ... }: {
    proc.groups.services.processes =
      let
        postgresql = pkgs.writeShellApplication {
          name = "postgresql";
          runtimeInputs = [ pkgs.postgresql_14 ];
          text = ''
            mkdir -p ${runtimeDir}/nxy
            if ! [ -d "$PGDATA" ]; then 
              initdb
            fi
            exec postgres --listen-addresses="" --unix_socket_directories=${runtimeDir}/nxy
          '';
        };
        nixServeSocket = "${runtimeDir}/nxy/nix_serve.sock";
        caddyConfig = pkgs.writeText "caddy-config" ''
          {
            log {
              output stdout
              format console
            }
          }
          http://localhost:8080
          reverse_proxy http://localhost:8085
          reverse_proxy /nar/* unix//${nixServeSocket}
          reverse_proxy /*.narinfo unix//${nixServeSocket}
          reverse_proxy /nix-cache-info unix//${nixServeSocket}
        '';
      in
      {
        postgresql.command = "${lib.getExe postgresql}";
        nix-serve.command = "${lib.getExe pkgs.nix-serve} --listen ${nixServeSocket}";
        caddy.command = "${lib.getExe pkgs.caddy} run --adapter=caddyfile --config ${caddyConfig}";
      };
  };
}
