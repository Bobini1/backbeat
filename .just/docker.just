# Build the Backbeat CLI Docker image.
docker-build IMAGE_NAME:
	#!/bin/bash
	set -euo pipefail
	DOCKER_BUILDKIT=1 docker build -t {{IMAGE_NAME}}:latest \
		-f docker/Dockerfile.{{IMAGE_NAME}} \
		--build-arg "OCI_CREATED=$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
		--build-arg "OCI_REVISION=$(git rev-parse HEAD)" \
		.
