#!/usr/bin/env bash
# Smoke test: runs one query against every table to verify the example works.
# Run after setup.sh has completed.
set -euo pipefail
cd "$(dirname "$0")/../.."

PASS=0
FAIL=0

psql_cmd() {
  docker compose -f examples/threads/docker-compose.yml exec -T -e PGPASSWORD=postgres db psql -U postgres -P pager=off "$@"
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

echo "=== Threads API FDW Example Verification ==="
echo ""

# 1. My Profile — single object, query param auth
echo "My Profile (single object):"
run_test "Fetch profile" \
  "SELECT id, username, name FROM my_profile;" \
  "(1 row)"
run_test "Has username" \
  "SELECT username FROM my_profile;" \
  "username"

# 2. My Threads — paginated list, cursor-based pagination
echo ""
echo "My Threads (pagination + timestamptz):"
run_test "Basic query" \
  "SELECT id, text, media_type, timestamp FROM my_threads LIMIT 5;" \
  "id"
run_count_test "Has posts" \
  "SELECT count(*) FROM (SELECT 1 FROM my_threads LIMIT 5) t;" \
  1

# 3. My Replies — same pagination pattern
echo ""
echo "My Replies:"
run_test "Basic query" \
  "SELECT id, text, timestamp FROM my_replies LIMIT 5;" \
  "id"

# 4. Thread Detail — path parameter substitution
echo ""
echo "Thread Detail (path param):"
# Get a thread_id from my_threads first
THREAD_ID=$(psql_cmd -t -c "SELECT id FROM my_threads LIMIT 1;" 2>/dev/null | tr -d ' \n')
if [ -n "$THREAD_ID" ]; then
  run_test "Fetch by ID" \
    "SELECT id, text, media_type FROM thread_detail WHERE thread_id = '$THREAD_ID';" \
    "(1 row)"
else
  echo "  SKIP (no threads found)"
fi

# 5. Thread Replies — path param + pagination
echo ""
echo "Thread Replies (path param + pagination):"
if [ -n "$THREAD_ID" ]; then
  run_test "Fetch replies" \
    "SELECT id, text, username FROM thread_replies WHERE thread_id = '$THREAD_ID' LIMIT 5;" \
    "id"
else
  echo "  SKIP (no threads found)"
fi

# 6. Thread Conversation — flattened all-depth replies
echo ""
echo "Thread Conversation (all-depth replies):"
if [ -n "$THREAD_ID" ]; then
  run_test "Fetch conversation" \
    "SELECT id, text, username FROM thread_conversation WHERE thread_id = '$THREAD_ID' LIMIT 5;" \
    "id"
else
  echo "  SKIP (no threads found)"
fi

# 7. Keyword Search — query param pushdown
echo ""
echo "Keyword Search (query param pushdown):"
run_test "Search for 'threads'" \
  "SELECT id, text, username FROM keyword_search WHERE q = 'threads' LIMIT 5;" \
  "id"

# 8. Profile Lookup — query param pushdown, single object
# Note: requires threads_basic permission which not all tokens have
echo ""
echo "Profile Lookup (query param):"
printf "  %-40s " "Look up @threads"
pl_output=$(psql_cmd -c "SELECT username, name, is_verified FROM profile_lookup WHERE username = 'threads';" 2>&1) || true
if echo "$pl_output" | grep -q "threads"; then
  echo "PASS"
  PASS=$((PASS + 1))
elif echo "$pl_output" | grep -qi "error\|permission\|500"; then
  echo "SKIP (permission not available)"
else
  echo "FAIL"
  echo "    Expected: threads"
  echo "    Output:   $(echo "$pl_output" | head -5)"
  FAIL=$((FAIL + 1))
fi

# 9. Publishing Limit — nested data response
echo ""
echo "Publishing Limit:"
run_test "Fetch quota" \
  "SELECT quota_usage, config FROM publishing_limit;" \
  "quota_usage"

# 10. Debug Mode
echo ""
echo "Debug Mode:"
run_test "HTTP request details" \
  "SELECT id FROM keyword_search_debug WHERE q = 'meta' LIMIT 1;" \
  "HTTP GET"

# 11. IMPORT FOREIGN SCHEMA
echo ""
echo "IMPORT FOREIGN SCHEMA:"
psql_cmd -c "DROP SCHEMA IF EXISTS threads_auto CASCADE;" > /dev/null 2>&1
psql_cmd -c "CREATE SCHEMA threads_auto;" > /dev/null 2>&1
run_test "Auto-generate tables" \
  "IMPORT FOREIGN SCHEMA \"unused\" FROM SERVER threads_import INTO threads_auto;" \
  "IMPORT FOREIGN SCHEMA"
run_count_test "Generated tables" \
  "SELECT count(*) FROM information_schema.foreign_tables WHERE foreign_table_schema = 'threads_auto';" \
  1
psql_cmd -c "DROP SCHEMA threads_auto CASCADE;" > /dev/null 2>&1

# 12. Attrs catch-all
echo ""
echo "Attrs catch-all column:"
run_test "Extra fields in attrs" \
  "SELECT id, attrs->>'media_product_type' AS product_type FROM my_threads LIMIT 3;" \
  "THREADS"

# Summary
echo ""
echo "============================================"
echo "  Results: $PASS passed, $FAIL failed"
echo "============================================"

[ "$FAIL" -eq 0 ]
