#!/usr/bin/env python3
"""ci/catalog-test-targets.py — Static analysis of C++ test targets.

Parses BUILD.bazel files under src/ray/ to extract ray_cc_test targets and
their key properties. Outputs a categorized summary useful for test triage
planning.

Usage:
    python3 ci/catalog-test-targets.py [--csv] [<repo-root>]

Options:
    --csv       Output raw CSV instead of formatted summary
    <repo-root> Path to the repository root (default: ".")
"""

import os
import re
import sys
from collections import defaultdict
from pathlib import Path


def find_build_files(root: str):
    """Find all BUILD.bazel files under src/ray/."""
    src_ray = Path(root) / "src" / "ray"
    return sorted(src_ray.rglob("BUILD.bazel"))


def extract_tests(build_file: Path, repo_root: str):
    """Extract ray_cc_test targets from a BUILD.bazel file."""
    content = build_file.read_text()
    package = str(build_file.parent.relative_to(repo_root))
    tests = []

    # Find all ray_cc_test( blocks
    # We need to handle nested parentheses properly
    pattern = re.compile(r"ray_cc_test\(")
    for m in pattern.finditer(content):
        start = m.start()
        # Find matching close paren
        depth = 0
        i = m.end() - 1  # position of opening paren
        for i in range(m.end() - 1, len(content)):
            if content[i] == "(":
                depth += 1
            elif content[i] == ")":
                depth -= 1
                if depth == 0:
                    break
        block = content[start : i + 1]

        # Extract attributes
        name = _extract_string(block, "name")
        size = _extract_string(block, "size") or "default"
        tags = _extract_list(block, "tags")
        uses_redis = "redis-server" in block or "redis-cli" in block
        has_exclusive = "exclusive" in tags

        if name:
            tests.append(
                {
                    "package": package,
                    "name": name,
                    "size": size,
                    "tags": tags,
                    "uses_redis": uses_redis,
                    "has_exclusive": has_exclusive,
                }
            )

    return tests


def _extract_string(block: str, attr: str):
    """Extract a string attribute value from a Bazel rule block."""
    m = re.search(rf'{attr}\s*=\s*"([^"]*)"', block)
    return m.group(1) if m else None


def _extract_list(block: str, attr: str):
    """Extract a list attribute from a Bazel rule block."""
    m = re.search(rf"{attr}\s*=\s*\[([^\]]*)\]", block, re.DOTALL)
    if not m:
        return []
    items_str = m.group(1)
    return [s.strip().strip('"') for s in items_str.split(",") if s.strip().strip('"')]


# Known categories for failure prediction
REDIS_TESTS = {
    "gcs_server_rpc_test",
    "gcs_kv_manager_test",
    "redis_gcs_table_storage_test",
    "redis_store_client_test",
    "chaos_redis_store_client_test",
    "redis_async_context_test",
    "global_state_accessor_test",
    "gcs_client_test",
    "gcs_client_reconnection_test",
}


def classify_test(test: dict) -> str:
    """Classify a test into a predicted category."""
    tags = test["tags"]
    name = test["name"]

    if "cgroup" in tags:
        return "cgroup (filtered out)"
    if test["uses_redis"] or name in REDIS_TESTS:
        return "redis-dependent"
    if any(t in tags for t in ["no_tsan", "no_ubsan"]):
        return "sanitizer-sensitive"
    return "standard"


def print_csv(all_tests):
    """Print all tests as CSV."""
    print("package,name,size,tags,uses_redis,category")
    for t in all_tests:
        tags_str = ";".join(t["tags"]) if t["tags"] else ""
        cat = classify_test(t)
        print(f"{t['package']},{t['name']},{t['size']},{tags_str},{t['uses_redis']},{cat}")


def print_summary(all_tests):
    """Print a formatted summary."""
    by_category = defaultdict(list)
    by_subsystem = defaultdict(list)

    for t in all_tests:
        cat = classify_test(t)
        by_category[cat].append(t)

        # Extract subsystem from package path
        parts = t["package"].split("/")
        # e.g., src/ray/gcs/tests -> gcs, src/ray/core_worker/tests -> core_worker
        if len(parts) >= 3:
            subsystem = parts[2]  # src/ray/<subsystem>
        else:
            subsystem = "other"
        by_subsystem[subsystem].append(t)

    total = len(all_tests)
    print(f"## C++ Test Target Catalog")
    print()
    print(f"**Total test targets:** {total}")
    print()

    # Category summary
    print("### By Category")
    print()
    print("| Category | Count | Description |")
    print("|----------|-------|-------------|")
    cat_desc = {
        "standard": "Expected to pass without special infrastructure",
        "redis-dependent": "Requires redis-server and redis-cli binaries (see #80)",
        "cgroup (filtered out)": "Filtered by --test_tag_filters=-cgroup",
        "sanitizer-sensitive": "Has no_tsan/no_ubsan tags (may need care under sanitizers)",
    }
    for cat in ["standard", "redis-dependent", "sanitizer-sensitive", "cgroup (filtered out)"]:
        tests = by_category.get(cat, [])
        desc = cat_desc.get(cat, "")
        if tests:
            print(f"| {cat} | {len(tests)} | {desc} |")
    print()

    # Subsystem summary
    print("### By Subsystem")
    print()
    print("| Subsystem | Total | Redis | Standard |")
    print("|-----------|-------|-------|----------|")
    for sub in sorted(by_subsystem.keys()):
        tests = by_subsystem[sub]
        redis_count = sum(1 for t in tests if classify_test(t) == "redis-dependent")
        standard_count = sum(1 for t in tests if classify_test(t) == "standard")
        print(f"| {sub} | {len(tests)} | {redis_count} | {standard_count} |")
    print()

    # Redis-dependent tests (detail)
    redis_tests = by_category.get("redis-dependent", [])
    if redis_tests:
        print("### Redis-Dependent Tests (require #80)")
        print()
        for t in redis_tests:
            print(f"- `{t['package']}:{t['name']}`")
        print()

    # Cgroup tests (detail)
    cgroup_tests = by_category.get("cgroup (filtered out)", [])
    if cgroup_tests:
        print("### Cgroup Tests (filtered out by -cgroup)")
        print()
        for t in cgroup_tests:
            print(f"- `{t['package']}:{t['name']}`")
        print()

    # Tests with exclusive tag
    exclusive_tests = [t for t in all_tests if t["has_exclusive"]]
    if exclusive_tests:
        print("### Tests with `exclusive` Tag")
        print()
        for t in exclusive_tests:
            print(f"- `{t['package']}:{t['name']}`")
        print()

    # Size distribution
    sizes = defaultdict(int)
    for t in all_tests:
        sizes[t["size"]] += 1
    print("### Size Distribution")
    print()
    print("| Size | Count |")
    print("|------|-------|")
    for size in ["small", "medium", "default"]:
        if size in sizes:
            print(f"| {size} | {sizes[size]} |")
    print()


def main():
    csv_mode = "--csv" in sys.argv
    args = [a for a in sys.argv[1:] if not a.startswith("-")]
    repo_root = args[0] if args else "."

    build_files = find_build_files(repo_root)
    all_tests = []
    for bf in build_files:
        all_tests.extend(extract_tests(bf, repo_root))

    if csv_mode:
        print_csv(all_tests)
    else:
        print_summary(all_tests)


if __name__ == "__main__":
    main()
