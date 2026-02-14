#!/usr/bin/env bash
# Smoke test: runs one query against every table to verify the example works.
# Run after setup.sh has completed.
set -euo pipefail
cd "$(dirname "$0")/../.."

PASS=0
FAIL=0

psql_cmd() {
  docker compose -f examples/carapi/docker-compose.yml exec -T -e PGPASSWORD=postgres db psql -U postgres -P pager=off "$@"
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

echo "=== CarAPI FDW Example Verification ==="
echo ""

# 1. Makes — paginated list, auto-detected "data" wrapper
echo "Makes (pagination + auto-detected wrapper):"
run_test "Basic query" \
  "SELECT id, name FROM makes LIMIT 5;" \
  "Acura"
run_count_test "Has makes" \
  "SELECT count(*) FROM (SELECT 1 FROM makes LIMIT 10) t;" \
  5

# 2. Models — query param pushdown (make, year)
echo ""
echo "Models (query pushdown):"
run_test "Toyota 2020 models" \
  "SELECT id, name, make FROM models WHERE make = 'Toyota' AND year = '2020' LIMIT 5;" \
  "Toyota"

# 3. Trims — query pushdown, pricing data
echo ""
echo "Trims (pricing + query pushdown):"
run_test "2020 Camry trims" \
  "SELECT trim, msrp, description FROM trims WHERE year = '2020' AND make = 'Toyota' AND model = 'Camry' LIMIT 3;" \
  "Sedan"

# 4. Bodies — vehicle dimensions
echo ""
echo "Bodies (dimensions):"
run_test "2020 Camry body" \
  "SELECT type, doors, length, curb_weight FROM bodies WHERE year = '2020' AND make = 'Toyota' AND model = 'Camry' LIMIT 2;" \
  "Sedan"

# 5. Engines — performance specs
echo ""
echo "Engines (performance data):"
run_test "2020 Camry engines" \
  "SELECT engine_type, horsepower_hp, cylinders, transmission FROM engines WHERE year = '2020' AND make = 'Toyota' AND model = 'Camry' LIMIT 2;" \
  "horsepower_hp"

# 6. Mileages — fuel economy
echo ""
echo "Mileages (fuel economy):"
run_test "2020 Camry mileage" \
  "SELECT combined_mpg, epa_city_mpg, epa_highway_mpg, range_city FROM mileages WHERE year = '2020' AND make = 'Toyota' AND model = 'Camry' LIMIT 2;" \
  "combined_mpg"

# 7. Exterior Colors — color + RGB
echo ""
echo "Exterior Colors (color data):"
run_test "2020 Camry colors" \
  "SELECT color, rgb FROM exterior_colors WHERE year = '2020' AND make = 'Toyota' AND model = 'Camry' LIMIT 3;" \
  "color"

# 8. OBD Codes — diagnostic codes
echo ""
echo "OBD Codes:"
run_test "Fetch codes" \
  "SELECT code, description FROM obd_codes LIMIT 5;" \
  "code"

# 9. Debug Mode
echo ""
echo "Debug Mode:"
run_test "HTTP request details" \
  "SELECT id FROM makes_debug LIMIT 1;" \
  "HTTP GET"

# Summary
echo ""
echo "============================================"
echo "  Results: $PASS passed, $FAIL failed"
echo "============================================"

[ "$FAIL" -eq 0 ]
