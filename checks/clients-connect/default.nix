{ runTests, ... }: {
  checks.clients-connect = runTests {
    name = "nxy-clients-connect";

    nxy.test.testScript = builtins.readFile ./test-script.py;
  };
}
