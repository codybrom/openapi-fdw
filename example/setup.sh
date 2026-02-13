#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> Building WASM binary..."
make build
chmod +r target/wasm32-unknown-unknown/release/openapi_fdw.wasm

echo ""
echo "==> Starting PostgreSQL..."
docker compose -f example/docker-compose.yml down -v 2>/dev/null || true
docker compose -f example/docker-compose.yml up -d

echo "Waiting for PostgreSQL..."
for i in $(seq 1 60); do
  if docker compose -f example/docker-compose.yml exec -T db pg_isready -U supabase_admin > /dev/null 2>&1; then
    echo "PostgreSQL ready after ${i}s"
    break
  fi
  if [ "$i" -eq 60 ]; then
    echo "ERROR: PostgreSQL failed to start"
    exit 1
  fi
  sleep 1
done
sleep 3  # wait for init scripts to complete

echo ""
echo "==> Copying WASM binary into container..."
container=$(docker compose -f example/docker-compose.yml ps -q db)
docker cp target/wasm32-unknown-unknown/release/openapi_fdw.wasm "$container":/openapi_fdw.wasm
docker compose -f example/docker-compose.yml exec -T db chmod 644 /openapi_fdw.wasm

echo ""
echo "============================================"
echo "  Ready! Connect with:"
echo "  psql postgresql://postgres:postgres@localhost:54322/postgres"
echo "============================================"
