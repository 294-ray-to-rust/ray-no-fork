# Project Memory

## Status
- **Project status**: in_progress
- **Last run**: 2026-02-23T19:36:58Z
- **Run count**: 6

## Summary
Scheduler ABI definition and Bazel alignment are the only ready paths. Scheduler FFI scaffolding is blocked until the Bazel toolchain mismatch is resolved.

## Completed Issues
- #1: Set up Rust raylet crate
- #2: Raylet Rust rewrite plan
- #9: Add raylet-rs CI checks

## Active Issues
- #5: Bridge C++ raylet main to Rust FFI [blocked]
- #6: Port LocalResourceManager logic to Rust [blocked]
- #7: Define ABI-safe raylet scheduling data model [ready]
- #8: Bootstrap scheduler FFI scaffolding [blocked]
- #10: Port ClusterResourceScheduler core to Rust [blocked]
- #21: Align Bazel version for raylet Rust tests [ready]

## Decisions
- Reset #7 to ready after it remained in-progress across multiple cycles.
- Removed the ready label from #8 and confirmed it is blocked on #21.
- Kept #5, #6, and #10 blocked until #21 and #7 resolve their dependencies.

## Blockers
- #5 is blocked by the Bazel version mismatch (needs #21).
- #6 and #10 require the ABI structs from #7 before implementation can proceed.
- #8 requires Bazel toolchain alignment in #21 to run validation.

## Next Priorities
1. Finish #7 to define the ABI structs.
2. Complete #21 to align Bazel tooling and unblock #5.
3. Unblock #8 once #21 lands and resume scheduler FFI scaffolding.
