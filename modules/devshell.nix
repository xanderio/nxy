{ self, ... }: {
  perSystem = { pkgs, config, rustPkgs, runtimeDir, ... }:
    {
      _module.args.runtimeDir =
        let runtimeDir = builtins.getEnv "XDG_RUNTIME_DIR";
        in if runtimeDir == "" then ".pg" else runtimeDir;

      devShells.default =
        rustPkgs.workspaceShell {
          RUSTFLAGS = "--cfg tokio_unstable";

          PGDATA = ".pg/data";
          PGHOST = "${runtimeDir}/nxy";
          PGDATABASE = "nxy";
          DATABASE_URL = "postgres://";

          nativeBuildInputs = with pkgs; [
            # runtime deps
            postgresql_14
            nix-serve

            config.proc.groups.services.package

            # developer tooling
            python3Packages.pgcli
            sqlx-cli
            websocat
          ];
        };
    };
}
