#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."

docker compose -f examples/nws/docker-compose.yml down -v
