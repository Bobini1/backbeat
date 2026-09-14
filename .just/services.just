compose_file := "docker-compose-localserver.yml"

# Start the local data server (8080) and collection server (8081).
local-servers:
	#!/bin/bash
	set -euo pipefail
	DOCKER_BUILDKIT=1 docker compose -f {{compose_file}} up --build -d

# Stop and remove the local server containers.
local-servers-down:
	#!/bin/bash
	set -euo pipefail
	docker compose -f {{compose_file}} down
