#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."

docker compose -f examples/threads/docker-compose.yml down -v
