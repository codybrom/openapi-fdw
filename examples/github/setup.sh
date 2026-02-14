#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."

# Load .env if present
if [ -f "examples/github/.env" ]; then
  set -a
  source examples/github/.env
  set +a
fi

if [ -z "${GITHUB_TOKEN:-}" ]; then
  echo "ERROR: Set GITHUB_TOKEN in examples/github/.env or as an env var."
  echo ""
  echo "  cp examples/github/.env.example examples/github/.env"
  echo "  # edit .env with your token"
  echo "  ./examples/github/setup.sh"
  exit 1
fi

echo "==> Building WASM binary..."
make build
chmod +r target/wasm32-unknown-unknown/release/openapi_fdw.wasm

echo ""
echo "==> Starting PostgreSQL..."
docker compose -f examples/github/docker-compose.yml down -v 2>/dev/null || true
docker compose -f examples/github/docker-compose.yml up -d

echo "Waiting for PostgreSQL..."
for i in $(seq 1 60); do
  if docker compose -f examples/github/docker-compose.yml exec -T db pg_isready -U supabase_admin > /dev/null 2>&1; then
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
container=$(docker compose -f examples/github/docker-compose.yml ps -q db)
docker cp target/wasm32-unknown-unknown/release/openapi_fdw.wasm "$container":/openapi_fdw.wasm
docker compose -f examples/github/docker-compose.yml exec -T db chmod 644 /openapi_fdw.wasm

# Replace placeholder api_key with the real access token
docker compose -f examples/github/docker-compose.yml exec -T \
  -e PGPASSWORD=postgres db \
  psql -U supabase_admin -d postgres -c "
    ALTER SERVER github OPTIONS (SET api_key '${GITHUB_TOKEN}');
    ALTER SERVER github_debug OPTIONS (SET api_key '${GITHUB_TOKEN}');
  "

echo ""
echo "============================================"
echo "  Ready! Connect with:"
echo "  psql postgresql://postgres:postgres@localhost:54324/postgres"
echo "============================================"
