#!/usr/bin/env bash
# Smoke test: runs one query against every table to verify the example works.
# Run after setup.sh has completed.
set -euo pipefail
cd "$(dirname "$0")/.."

PASS=0
FAIL=0

psql_cmd() {
  docker compose -f example/docker-compose.yml exec -T -e PGPASSWORD=postgres db psql -U postgres -P pager=off "$@"
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

echo "=== OpenAPI FDW Example Verification ==="
echo ""

# 1. Stations — GeoJSON + pagination + camelCase matching
echo "Stations (GeoJSON + pagination):"
run_test "Basic query" \
  "SELECT station_identifier, name, time_zone FROM stations LIMIT 3;" \
  "station_identifier"
run_count_test "Pagination (50+ rows fetched)" \
  "SELECT count(*) FROM (SELECT 1 FROM stations LIMIT 60) t;" \
  51
run_test "Lookup by rowid_column" \
  "SELECT station_identifier, name FROM stations WHERE station_identifier = 'KDEN';" \
  "KDEN"

# 1b. Stations — JSONB column (elevation is a structured object)
echo ""
echo "JSONB Columns:"
run_test "Elevation as jsonb" \
  "SELECT station_identifier, elevation->>'value' AS elev, elevation->>'unitCode' AS unit FROM stations LIMIT 3;" \
  "wmoUnit"

# 1d. camelCase → snake_case matching
echo ""
echo "camelCase Matching:"
run_test "stationIdentifier → station_identifier" \
  "SELECT station_identifier FROM stations LIMIT 1;" \
  "station_identifier"
run_test "timeZone → time_zone" \
  "SELECT time_zone FROM stations WHERE time_zone IS NOT NULL LIMIT 1;" \
  "(1 row)"

# 2. Active alerts — timestamptz coercion
echo ""
echo "Active Alerts (timestamptz coercion):"
run_test "Alerts with timestamps" \
  "SELECT event, severity, headline, onset FROM active_alerts LIMIT 3;" \
  "severity"
run_test "timestamptz format (onset)" \
  "SELECT onset FROM active_alerts WHERE onset IS NOT NULL LIMIT 1;" \
  "+00"

# 3. Query param pushdown
echo ""
echo "Query Param Pushdown (severity=Severe):"
run_test "Filter by severity" \
  "SELECT event, severity, headline FROM active_alerts WHERE severity = 'Severe' LIMIT 3;" \
  "severity"

# 4. Path parameter — station observations
echo ""
echo "Station Observations (path param):"
run_test "KDEN observations" \
  "SELECT timestamp, text_description, temperature->>'value' AS temp FROM station_observations WHERE station_id = 'KDEN' LIMIT 3;" \
  "text_description"

# 5. Single object — latest observation
echo ""
echo "Latest Observation (single object):"
run_test "Single row response" \
  "SELECT text_description, temperature->>'value' AS temp FROM latest_observation WHERE station_id = 'KDEN';" \
  "(1 row)"

# 6. Composite path param — point metadata
echo ""
echo "Point Metadata (composite path param):"
run_test "Denver coordinates" \
  "SELECT grid_id, grid_x, grid_y FROM point_metadata WHERE point = '39.7456,-104.9887';" \
  "BOU"

# 7. Multiple path params + nested response_path — forecast
echo ""
echo "Forecast (multi-path-param + nested response):"
run_test "Denver forecast" \
  "SELECT name, temperature, temperature_unit, is_daytime, short_forecast FROM forecast_periods WHERE wfo = 'BOU' AND x = '63' AND y = '62' LIMIT 3;" \
  "temperature_unit"

# 7b. Type coercion in forecast
echo ""
echo "Type Coercion:"
run_test "Boolean (is_daytime)" \
  "SELECT is_daytime FROM forecast_periods WHERE wfo = 'BOU' AND x = '63' AND y = '62' LIMIT 1;" \
  "t"
run_test "Integer (temperature)" \
  "SELECT temperature FROM forecast_periods WHERE wfo = 'BOU' AND x = '63' AND y = '62' LIMIT 1;" \
  "(1 row)"

# 7c. LIMIT pushdown (stops pagination early)
echo ""
echo "LIMIT Pushdown:"
run_count_test "LIMIT 3 returns exactly 3" \
  "SELECT count(*) FROM (SELECT 1 FROM stations LIMIT 3) t;" \
  3

# 8. Debug mode
echo ""
echo "Debug Mode:"
run_test "HTTP request details" \
  "SELECT station_identifier FROM stations_debug LIMIT 1;" \
  "HTTP GET"
run_test "Scan statistics" \
  "SELECT station_identifier FROM stations_debug LIMIT 1;" \
  "Scan complete"

# 9. IMPORT FOREIGN SCHEMA
echo ""
echo "IMPORT FOREIGN SCHEMA:"
psql_cmd -c "DROP SCHEMA IF EXISTS nws_verify CASCADE;" > /dev/null 2>&1
psql_cmd -c "CREATE SCHEMA nws_verify;" > /dev/null 2>&1
run_test "Auto-generate tables" \
  "IMPORT FOREIGN SCHEMA \"unused\" FROM SERVER nws_import INTO nws_verify;" \
  "IMPORT FOREIGN SCHEMA"
run_count_test "Generated tables" \
  "SELECT count(*) FROM information_schema.foreign_tables WHERE foreign_table_schema = 'nws_verify';" \
  1
psql_cmd -c "DROP SCHEMA nws_verify CASCADE;" > /dev/null 2>&1

# 10. attrs catch-all
echo ""
echo "Attrs catch-all column:"
run_test "Extra fields in attrs" \
  "SELECT station_identifier, attrs->>'county' AS county FROM stations LIMIT 3;" \
  "county"

# Summary
echo ""
echo "============================================"
echo "  Results: $PASS passed, $FAIL failed"
echo "============================================"

[ "$FAIL" -eq 0 ]
