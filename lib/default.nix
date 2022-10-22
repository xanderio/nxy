{ self, final, system }: {
  activate = rec {
    custom =
      {
        __functor = customSelf: base: activate:
          final.buildEnv {
            name = ("activatable-" + base.name);
            paths =
              [
                base
                (final.writeTextFile {
                  name = base.name + "-activate-path";
                  text = ''
                    #!${final.runtimeShell}
                    set -euo pipefail

                    if [[ "''${DRY_ACTIVATE:-}" == "1" ]]; then
                        ${customSelf.dryActivate or "echo ${final.writeScript "activate" activate}"}
                    else
                        ${activate}
                    fi
                  '';
                  executable = true;
                  destination = "/deploy-rs-activate";
                })
                (final.writeTextFile {
                  name = base.name + "-activate-rs";
                  text = ''
                    #!${final.runtimeShell}
                    exec ${self.packages.${system}.deploy}/bin/activate "$@"
                  '';
                  executable = true;
                  destination = "/activate-rs";
                })
              ];
          };
      };

    nixos = base:
      (custom // { dryActivate = "$PROFILE/bin/switch-to-configuration dry-activate"; })
        base.config.system.build.toplevel ''
        # work around https://github.com/NixOS/nixpkgs/issues/73404
        cd /tmp

        $PROFILE/bin/switch-to-configuration switch

        # https://github.com/serokell/deploy-rs/issues/31
        ${with base.config.boot.loader;
        final.lib.optionalString systemd-boot.enable
        "sed -i '/^default /d' ${efi.efiSysMountPoint}/loader/loader.conf"}
      '';

    home-manager = base: custom base.activationPackage "$PROFILE/activate";

    noop = base: custom base ":";
  };

  deployChecks = deploy: builtins.mapAttrs (_: check: check deploy) {
    schema = deploy:
      let
        deploy-json = final.writeText "deploy.json" (builtins.toJSON deploy);
      in
      final.runCommand "jsonschema-deploy-system" { } ''
        ${final.python3.pkgs.jsonschema}/bin/jsonschema -i ${deploy-json} ${./crates/deploy/interface.json} && touch $out
      '';

    activate = deploy:
      let
        profiles = builtins.concatLists (final.lib.mapAttrsToList
          (nodeName: node: final.lib.mapAttrsToList
            (profileName: profile: [ (toString profile.path) nodeName profileName ])
            node.profiles)
          deploy.nodes);
      in
      final.runCommand "deploy-rs-check-activate" { } ''
        for x in ${builtins.concatStringsSep " " (map (p: builtins.concatStringsSep ":" p) profiles)}; do
          profile_path=$(echo $x | cut -f1 -d:)
          node_name=$(echo $x | cut -f2 -d:)
          profile_name=$(echo $x | cut -f3 -d:)

          if ! [[ -f "$profile_path/deploy-rs-activate" ]]; then
            echo "#$node_name.$profile_name is missing the deploy-rs-activate activation script" 
            exit 1
          fi

          if ! [[ -f "$profile_path/activate-rs" ]]; then
            echo "#$node_name.$profile_name is missing the activate-rs activation script" 
            exit 1
          fi
        done

        touch $out
      '';
  };
}
