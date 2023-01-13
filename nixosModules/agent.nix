self:
{ pkgs, lib, config, ... }:
let
  cfg = config.services.nxy-agent;
in
{
  options.services.nxy-agent = {
    enable = lib.mkEnableOption "nxy agent";

    server = lib.mkOption {
      description = "websocket url of nxy server";
      example = "ws://localhost:8080";
      type = lib.types.str;
    };
  };

  config = lib.mkIf cfg.enable {
    nixpkgs.overlays = [
      self.overlays.default
    ];


    systemd.services.nxy-agent = {
      enable = true;
      wantedBy = [ "multi-user.target" ];
      after = [ "nix-deamon.service" ];
      path = [ config.nix.package ];

      # don't stop the service if the unit disappers
      unitConfig.X-StopOnRemoval = false;

      serviceConfig = {
        # we don't want to kill childern processes as those are deployments
        KillMode = "process";
        Restart = "always";
        RestartSec = 5;
        ExecStart = "${pkgs.nxy-agent}/bin/nxy-agent /var/lib/nxy ${cfg.server}";
      };
    };
  };
}
