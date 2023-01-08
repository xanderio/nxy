self:
{ pkgs, lib, config, ... }:
let
  inherit (lib) types;
  cfg = config.services.nxy-server;

  json = pkgs.formats.json { };
in
{
  options.services.nxy-server = {
    enable = lib.mkEnableOption "nxy server";

    settings = lib.mkOption {
      type = lib.types.submodule {
        freeformType = json.type;
        options = {
          external_url = lib.mkOption {
            type = types.str;
          };
        };
      };
      default = { };
    };
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
        ExecStart = "${pkgs.nxy-server}/bin/nxy-server ${json.generate "nxy-server.json" cfg.settings}";
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
