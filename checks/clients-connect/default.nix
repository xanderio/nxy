{ runTests, ... }: {
  checks.clients-connect = runTests {
    name = "nxy-clients-connect";

    nxy.test = {
      flake = ./.;
      testScript = builtins.readFile ./test-script.py;
    };
  };
}
