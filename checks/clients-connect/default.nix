{ self, pkgs }:
let
  tools = pkgs.callPackage ../tools.nix { inherit self; };
in
tools.runTests {
  name = "nxy-clients-connect";

  nxy.test.testScript = builtins.readFile ./test-script.py;
}

