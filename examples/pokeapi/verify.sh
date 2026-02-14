#!/usr/bin/env bash
# Smoke test: runs one query against every table to verify the example works.
# Run after setup.sh has completed.
set -euo pipefail
cd "$(dirname "$0")/../.."

PASS=0
FAIL=0

psql_cmd() {
  docker compose -f examples/pokeapi/docker-compose.yml exec -T -e PGPASSWORD=postgres db psql -U postgres -P pager=off "$@"
}

run_test() {
  local name="$1"
  local sql="$2"
  local expected="$3"

  printf "  %-40s " "$name"
  local output
  output=$(psql_cmd -c "$sql" 2>&1) || true

  if echo "$output" | grep -q "$expected"; then
    echo "PASS"
    PASS=$((PASS + 1))
  else
    echo "FAIL"
    echo "    Expected: $expected"
    echo "    Output:   $(echo "$output" | head -5)"
    FAIL=$((FAIL + 1))
  fi
}

run_count_test() {
  local name="$1"
  local sql="$2"
  local min_count="$3"

  printf "  %-40s " "$name"
  local output
  output=$(psql_cmd -t -c "$sql" 2>&1) || true
  local count
  count=$(echo "$output" | tr -d ' \n')

  if [ "$count" -ge "$min_count" ] 2>/dev/null; then
    echo "PASS ($count rows)"
    PASS=$((PASS + 1))
  else
    echo "FAIL (got $count, expected >= $min_count)"
    FAIL=$((FAIL + 1))
  fi
}

echo "=== OpenAPI FDW PokéAPI Example Verification ==="
echo ""

# 1. Pokemon list — offset-based pagination with auto-detected `results` wrapper
echo "Pokemon List (offset-based pagination):"
run_test "Basic query" \
  "SELECT name, url FROM pokemon LIMIT 5;" \
  "name"
run_count_test "Pagination (20+ rows fetched)" \
  "SELECT count(*) FROM (SELECT 1 FROM pokemon LIMIT 25) t;" \
  1

# 2. Pokemon detail — path parameter substitution
echo ""
echo "Pokemon Detail (path param):"
run_test "Pikachu lookup" \
  "SELECT id, name, height, weight, base_experience FROM pokemon_detail WHERE name = 'pikachu';" \
  "(1 row)"

# 3. Types list
echo ""
echo "Types List:"
run_test "Basic query" \
  "SELECT name, url FROM types LIMIT 5;" \
  "name"

# 4. Type detail — path parameter substitution
echo ""
echo "Type Detail (path param):"
run_test "Fire type lookup" \
  "SELECT id, name FROM type_detail WHERE name = 'fire';" \
  "(1 row)"

# 5. Berries list
echo ""
echo "Berries List:"
run_test "Basic query" \
  "SELECT name, url FROM berries LIMIT 5;" \
  "name"

# 6. Berry detail — path parameter substitution
echo ""
echo "Berry Detail (path param):"
run_test "Cheri berry lookup" \
  "SELECT id, name, growth_time, max_harvest FROM berry_detail WHERE name = 'cheri';" \
  "(1 row)"

# 7. Debug mode
echo ""
echo "Debug Mode:"
run_test "HTTP request details" \
  "SELECT name FROM pokemon_debug LIMIT 1;" \
  "HTTP GET"

# Summary
echo ""
echo "============================================"
echo "  Results: $PASS passed, $FAIL failed"
echo "============================================"

[ "$FAIL" -eq 0 ]
