# Project Memory

## Status
- **Project status**: in_progress
- **Last run**: 2026-02-23T20:10:00Z
- **Run count**: 4

## Summary
Rust raylet crate exists and SWE is wiring the C++ entry plus scheduler components into the new FFI. LocalResourceManager and ABI/data-model work are back in the ready queue while scheduler FFI and the ClusterResourceScheduler remain the next milestones once those land.

## Completed Issues
- #1: Set up Rust raylet crate
- #2: Raylet Rust rewrite plan
- #9: Add raylet-rs CI checks

## Active Issues
- #5: Bridge C++ raylet main to Rust FFI [in-progress]
- #6: Port LocalResourceManager logic to Rust [ready]
- #7: Define ABI-safe raylet scheduling data model [ready]
- #8: Bootstrap scheduler FFI scaffolding [in-progress]
- #10: Port ClusterResourceScheduler core to Rust [blocked]

## Decisions
- Reset #6 and #7 to ready because they spanned multiple manager cycles without updates.
- Left #5 and #8 marked in-progress to reflect the active SWE claims started this cycle.
- Kept #10 blocked on #7 so the scheduler core does not start before the ABI is defined.
- Recorded completion of #9 so future work can assume CI exists for the crate.

## Blockers
- #10 requires the ABI structs from #7 before implementation can proceed.

## Next Priorities
1. Land #5 so the C++ raylet invokes the Rust crate under a feature flag.
2. Pick up #6 and #7 to port LocalResourceManager and define the ABI/structs.
3. After #7, continue with #8 and unblock #10 to move the scheduler loop to Rust.
