#!/usr/bin/env bash
# ci/triage-test-results.sh — Parse bazel test XML results and produce a
# categorized triage summary.
#
# Usage: ci/triage-test-results.sh <testlogs-dir>
#
# The script reads JUnit XML files from bazel-testlogs and categorizes each
# test target as PASSED, FAILED (with failure reason), or TIMEOUT.
# Output is a Markdown summary suitable for Buildkite annotations or issue
# comments.

set -euo pipefail

TESTLOGS_DIR="${1:?Usage: $0 <testlogs-dir>}"

if [ ! -d "$TESTLOGS_DIR" ]; then
  echo "Error: $TESTLOGS_DIR is not a directory" >&2
  exit 1
fi

# Counters
total=0
passed=0
failed=0
timed_out=0
no_result=0

# Arrays for categorized failures
declare -a redis_failures=()
declare -a timeout_failures=()
declare -a genuine_failures=()
declare -a infra_failures=()
declare -a passed_tests=()
declare -a no_result_tests=()

# Known redis-dependent tests
REDIS_TESTS=(
  "gcs_server_rpc_test"
  "gcs_kv_manager_test"
  "redis_gcs_table_storage_test"
  "redis_store_client_test"
  "chaos_redis_store_client_test"
  "redis_async_context_test"
  "global_state_accessor_test"
  "gcs_client_test"
  "gcs_client_reconnection_test"
)

is_redis_test() {
  local name="$1"
  for rt in "${REDIS_TESTS[@]}"; do
    if [[ "$name" == "$rt" ]]; then
      return 0
    fi
  done
  return 1
}

# Find all test.xml files
while IFS= read -r xml_file; do
  total=$((total + 1))

  # Extract test target path from directory structure
  # e.g., testlogs/src/ray/util/tests/array_test/test.xml -> //src/ray/util/tests:array_test
  rel_path="${xml_file#"$TESTLOGS_DIR"/}"
  dir_path="$(dirname "$rel_path")"
  test_name="$(basename "$dir_path")"
  package_path="$(dirname "$dir_path")"

  target="//${package_path}:${test_name}"

  # Parse XML for pass/fail status
  # Look for failures, errors, and time attributes
  if ! [ -s "$xml_file" ]; then
    no_result=$((no_result + 1))
    no_result_tests+=("$target")
    continue
  fi

  # Extract key attributes from the testsuite element
  tests_attr=$(grep -oP 'tests="\K[0-9]+' "$xml_file" 2>/dev/null | head -1 || echo "0")
  failures_attr=$(grep -oP 'failures="\K[0-9]+' "$xml_file" 2>/dev/null | head -1 || echo "0")
  errors_attr=$(grep -oP 'errors="\K[0-9]+' "$xml_file" 2>/dev/null | head -1 || echo "0")
  time_attr=$(grep -oP 'time="\K[0-9.]+' "$xml_file" 2>/dev/null | head -1 || echo "0")

  # Check for timeout markers in the XML
  has_timeout=false
  if grep -q 'TIMEOUT' "$xml_file" 2>/dev/null; then
    has_timeout=true
  fi

  if [ "$failures_attr" = "0" ] && [ "$errors_attr" = "0" ] && [ "$tests_attr" != "0" ]; then
    passed=$((passed + 1))
    passed_tests+=("$target")
  elif $has_timeout; then
    timed_out=$((timed_out + 1))
    timeout_failures+=("$target (${time_attr}s)")
  else
    failed=$((failed + 1))

    # Categorize the failure
    if is_redis_test "$test_name"; then
      redis_failures+=("$target")
    elif grep -qiE 'connection refused|address already in use|bind.*failed|port.*in use' "$xml_file" 2>/dev/null; then
      infra_failures+=("$target")
    else
      genuine_failures+=("$target")
    fi
  fi
done < <(find "$TESTLOGS_DIR" -name "test.xml" -type f | sort)

# Also check for test logs without XML (crashed before producing results)
while IFS= read -r log_file; do
  rel_path="${log_file#"$TESTLOGS_DIR"/}"
  dir_path="$(dirname "$rel_path")"
  test_name="$(basename "$dir_path")"
  xml_file="$dir_path/test.xml"

  # Skip if we already processed the XML
  if [ -f "$TESTLOGS_DIR/$xml_file" ]; then
    continue
  fi

  target="//${dir_path%%/test.log}:${test_name}"

  # Check if test log indicates a crash or infrastructure issue
  if grep -qiE 'killed|signal|segfault|oom' "$log_file" 2>/dev/null; then
    infra_failures+=("$target (crashed)")
    failed=$((failed + 1))
    total=$((total + 1))
  fi
done < <(find "$TESTLOGS_DIR" -name "test.log" -type f | sort)

# Count expected targets from catalog (if available)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
expected_total=""
if command -v python3 &>/dev/null && [ -f "$SCRIPT_DIR/catalog-test-targets.py" ]; then
  repo_root="$(cd "$SCRIPT_DIR/.." && pwd)"
  # Count non-cgroup tests (cgroup tests are filtered by --test_tag_filters=-cgroup)
  expected_total=$(python3 "$SCRIPT_DIR/catalog-test-targets.py" --csv "$repo_root" 2>/dev/null \
    | tail -n +2 \
    | grep -cv 'cgroup (filtered out)' || echo "")
fi

# Output summary
total_label="$total"
if [ -n "$expected_total" ] && [ "$expected_total" != "$total" ]; then
  total_label="$total / $expected_total expected"
fi

cat <<EOF
## C++ Test Triage Summary

| Category | Count |
|----------|-------|
| **Total test targets** | $total_label |
| ✅ Passed | $passed |
| ❌ Failed | $failed |
| ⏰ Timed out | $timed_out |
| ❓ No result | $no_result |

EOF

if [ ${#redis_failures[@]} -gt 0 ]; then
  echo "### 🔴 Redis-dependent failures (${#redis_failures[@]})"
  echo ""
  echo "These tests require \`redis-server\` and \`redis-cli\` at runtime."
  echo "See issue #80 for the fix."
  echo ""
  for t in "${redis_failures[@]}"; do
    echo "- \`$t\`"
  done
  echo ""
fi

if [ ${#timeout_failures[@]} -gt 0 ]; then
  echo "### ⏰ Timeout failures (${#timeout_failures[@]})"
  echo ""
  for t in "${timeout_failures[@]}"; do
    echo "- \`$t\`"
  done
  echo ""
fi

if [ ${#infra_failures[@]} -gt 0 ]; then
  echo "### 🟡 Infrastructure failures (${#infra_failures[@]})"
  echo ""
  echo "Tests that failed due to missing runtime deps, port conflicts, or crashes."
  echo ""
  for t in "${infra_failures[@]}"; do
    echo "- \`$t\`"
  done
  echo ""
fi

if [ ${#genuine_failures[@]} -gt 0 ]; then
  echo "### 🔴 Genuine test failures (${#genuine_failures[@]})"
  echo ""
  for t in "${genuine_failures[@]}"; do
    echo "- \`$t\`"
  done
  echo ""
fi

if [ ${#no_result_tests[@]} -gt 0 ]; then
  echo "### ❓ No result / empty XML (${#no_result_tests[@]})"
  echo ""
  for t in "${no_result_tests[@]}"; do
    echo "- \`$t\`"
  done
  echo ""
fi

if [ ${#passed_tests[@]} -gt 0 ]; then
  echo "<details>"
  echo "<summary>✅ Passed tests (${#passed_tests[@]})</summary>"
  echo ""
  for t in "${passed_tests[@]}"; do
    echo "- \`$t\`"
  done
  echo ""
  echo "</details>"
fi
