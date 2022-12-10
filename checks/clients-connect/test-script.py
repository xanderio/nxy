server.wait_until_succeeds(f"curl --fail http://localhost:8080/api/v1/agent | jq -e 'length == {len(clients)}'")
