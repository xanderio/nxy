{ self, pkgs }: {
  clients-connect = import ./clients-connect { inherit self pkgs; };
}
