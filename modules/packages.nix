{
  perSystem = { lib, commonArgs, craneLib, cargoArtifacts, ... }:
    {
      packages =
        let
          mkPackage = name: craneLib.buildPackage (commonArgs // {
            inherit cargoArtifacts;
            cargoExtraArgs = "--package ${name}";
          } // craneLib.crateNameFromCargoToml { src = ./../${name}; });
        in
        lib.genAttrs [ "nxy-server" "nxy-cli" "nxy-agent" ] mkPackage;
    };
}
