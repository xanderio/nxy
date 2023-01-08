server.wait_until_succeeds(f"curl -s --fail http://localhost/api/v1/agent | jq -e 'length == {len(clients)}'")

test_store_path = server.succeed("nix build --print-out-paths nixpkgs#hello").strip()

agent_id = alpha.succeed("jq '.id' /var/lib/nxy/state.json").strip()

with subtest("copy store path to agent"):
    alpha.fail(f"nix path-info {test_store_path}")
    server.succeed(f"NXY_SERVER=http://server nxy-cli agents download {agent_id} {test_store_path}")
    alpha.succeed(f"nix path-info {test_store_path}")
