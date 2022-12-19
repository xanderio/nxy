{ self, ... }: {
  perSystem = { pkgs, rustPkgs, ... }:
    {
      devShells.default =
        let
          xdg_runtime_dir =
            if builtins.getEnv "XDG_RUNTIME_DIR" == "" then
              ".pg"
            else
              builtins.getEnv "XDG_RUNTIME_DIR";
        in
        rustPkgs.workspaceShell {
          RUSTFLAGS = "--cfg tokio_unstable";

          PGDATA = ".pg/data";
          PGHOST = "${xdg_runtime_dir}/nxy";
          PGDATABASE = "nxy";
          DATABASE_URL = "postgres://";

          nativeBuildInputs = with pkgs; [
            # runtime deps
            postgresql_14

            # developer tooling
            python3Packages.pgcli
            sqlx-cli
            websocat
          ];
          shellHook = ''
            mkdir -p $XDG_RUNTIME_DIR/nxy
            if ! [ -d $PGDATA ]; then 
              initdb
            fi
            if ! pg_ctl status > /dev/null; then
              systemd-run --user --unit=nxy_postgres --service-type=notify \
                --same-dir -E PGDATA=$PGDATA \
                ${pkgs.postgresql_14}/bin/postgres --listen-addresses="" --unix_socket_directories=${xdg_runtime_dir}/nxy

                createdb
            fi
          '';
        };
    };
}
