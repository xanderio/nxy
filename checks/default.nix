{
  imports = [
    ./tools.nix
  ];
  perSystem = { self', ... }: {
    imports = [
      ./clients-connect
    ];
    checks = self'.packages;
  };
}
