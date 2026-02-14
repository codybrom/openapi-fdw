#!/usr/bin/env bash
# Smoke test: runs one query against every table to verify the example works.
# Run after setup.sh has completed.
set -euo pipefail
cd "$(dirname "$0")/../.."

PASS=0
FAIL=0

psql_cmd() {
  docker compose -f examples/github/docker-compose.yml exec -T -e PGPASSWORD=postgres db psql -U postgres -P pager=off "$@"
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

echo "=== GitHub API FDW Example Verification ==="
echo ""

# 1. My Profile — single object, bearer token auth
echo "My Profile (single object):"
run_test "Fetch profile" \
  "SELECT login, id, name FROM my_profile;" \
  "(1 row)"
run_test "Has login" \
  "SELECT login FROM my_profile;" \
  "login"

# 2. My Repos — paginated list, page-based pagination
echo ""
echo "My Repos (pagination):"
run_test "Basic query" \
  "SELECT id, name, language FROM my_repos LIMIT 5;" \
  "id"
run_count_test "Has repos" \
  "SELECT count(*) FROM (SELECT 1 FROM my_repos LIMIT 5) t;" \
  1

# 3. Repo Detail — two path parameters, single object
echo ""
echo "Repo Detail (path params):"
run_test "Fetch supabase/wrappers" \
  "SELECT name, stargazers_count, language FROM repo_detail WHERE owner = 'supabase' AND repo = 'wrappers';" \
  "(1 row)"

# 4. Repo Issues — path params + pagination + query pushdown
echo ""
echo "Repo Issues (path params + pagination):"
run_test "Fetch issues" \
  "SELECT number, title, state FROM repo_issues WHERE owner = 'supabase' AND repo = 'wrappers' LIMIT 5;" \
  "number"

# 5. Repo Pulls — path params + query pushdown (state=closed)
echo ""
echo "Repo Pulls (path params + state pushdown):"
run_test "Fetch closed PRs" \
  "SELECT number, title, state FROM repo_pulls WHERE owner = 'supabase' AND repo = 'wrappers' AND state = 'closed' LIMIT 5;" \
  "closed"

# 6. Repo Releases — path params + pagination
echo ""
echo "Repo Releases (path params):"
run_test "Fetch releases" \
  "SELECT tag_name, name, prerelease FROM repo_releases WHERE owner = 'supabase' AND repo = 'wrappers' LIMIT 5;" \
  "tag_name"

# 7. Search Repos — query pushdown, auto-detected items wrapper
echo ""
echo "Search Repos (query pushdown):"
run_test "Search for repos" \
  "SELECT name, full_name, stargazers_count FROM search_repos WHERE q = 'openapi foreign data wrapper' LIMIT 5;" \
  "name"

# 8. Debug Mode
echo ""
echo "Debug Mode:"
run_test "HTTP request details" \
  "SELECT id FROM search_repos_debug WHERE q = 'supabase' LIMIT 1;" \
  "HTTP GET"

# Summary
echo ""
echo "============================================"
echo "  Results: $PASS passed, $FAIL failed"
echo "============================================"

[ "$FAIL" -eq 0 ]
