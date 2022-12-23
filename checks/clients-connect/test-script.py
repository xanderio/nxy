server.wait_until_succeeds(f"curl -s --fail http://localhost/api/v1/agent | jq -e 'length == {len(clients)}'")
