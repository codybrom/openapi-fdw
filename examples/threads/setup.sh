#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."

# Load .env if present
if [ -f "examples/threads/.env" ]; then
  set -a
  source examples/threads/.env
  set +a
fi

if [ -z "${THREADS_ACCESS_TOKEN:-}" ]; then
  echo "ERROR: Set THREADS_ACCESS_TOKEN in examples/threads/.env or as an env var."
  echo ""
  echo "  cp examples/threads/.env.example examples/threads/.env"
  echo "  # edit .env with your token"
  echo "  ./examples/threads/setup.sh"
  exit 1
fi

echo "==> Building WASM binary..."
make build
chmod +r target/wasm32-unknown-unknown/release/openapi_fdw.wasm

echo ""
echo "==> Starting PostgreSQL..."
docker compose -f examples/threads/docker-compose.yml down -v 2>/dev/null || true
docker compose -f examples/threads/docker-compose.yml up -d

echo "Waiting for PostgreSQL..."
for i in $(seq 1 60); do
  if docker compose -f examples/threads/docker-compose.yml exec -T db pg_isready -U supabase_admin > /dev/null 2>&1; then
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
echo "==> Copying files into container..."
container=$(docker compose -f examples/threads/docker-compose.yml ps -q db)
docker cp target/wasm32-unknown-unknown/release/openapi_fdw.wasm "$container":/openapi_fdw.wasm
docker cp examples/threads/threads-openapi.json "$container":/threads-openapi.json
docker compose -f examples/threads/docker-compose.yml exec -T db chmod 644 /openapi_fdw.wasm /threads-openapi.json

# Start a lightweight HTTP server to serve the OpenAPI spec for IMPORT FOREIGN SCHEMA
docker compose -f examples/threads/docker-compose.yml exec -T -d db \
  python3 -m http.server 8888 --directory /

# Replace placeholder api_key with the real access token
docker compose -f examples/threads/docker-compose.yml exec -T \
  -e PGPASSWORD=postgres db \
  psql -U supabase_admin -d postgres -c "
    ALTER SERVER threads OPTIONS (SET api_key '${THREADS_ACCESS_TOKEN}');
    ALTER SERVER threads_debug OPTIONS (SET api_key '${THREADS_ACCESS_TOKEN}');
    ALTER SERVER threads_import OPTIONS (SET api_key '${THREADS_ACCESS_TOKEN}');
  "

echo ""
echo "============================================"
echo "  Ready! Connect with:"
echo "  psql postgresql://postgres:postgres@localhost:54323/postgres"
echo "============================================"
