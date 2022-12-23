self:
{ pkgs, lib, config, ... }:
let
  cfg = config.services.nxy-server;
in
{
  options.services.nxy-server = {
    enable = lib.mkEnableOption "nxy server";
  };

  config = lib.mkIf cfg.enable {
    nixpkgs.overlays = [
      self.overlays.default
    ];

    services.postgresql = {
      enable = true;
      ensureDatabases = [ "nxy" ];
      ensureUsers = [
        {
          name = "nxy";
          ensurePermissions = {
            "DATABASE nxy" = "ALL PRIVILEGES";
          };
        }
      ];
    };

    users.users.nxy = {
      isSystemUser = true;
      group = "nxy";
    };

    users.groups.nxy = { };

    systemd.services.nxy-server = {
      enable = true;
      wantedBy = [ "multi-user.target" ];
      after = [ "postgresql.service" ];
      requires = [ "postgresql.service" ];
      serviceConfig = {
        ExecStart = "${pkgs.nxy-server}/bin/nxy-server";
        User = "nxy";
        Group = "nxy";
      };
      environment = {
        PGHOST = "/var/run/postgresql";
        PGAPPNAME = "nxy";
      };
    };
  };
}
