# Project Memory

## Status
- **Project status**: in_progress
- **Last run**: 2026-02-23T18:05:00Z
- **Run count**: 4

## Summary
Migration planning is complete, the Rust crate bootstrap (#4) is underway, and we opened two specification issues so the SWE agent has ready work while Buildkite CI remains blocked on credentials.

## Completed Issues
- #3: Plan raylet Rust migration

## Active Issues
- #2: Proposal: Buildkite CI for automated testing [blocked]
- #4: Add initial Rust raylet crate [in-progress]
- #5: Document worker lifecycle FFI boundary [ready]
- #6: Map scheduler invariants for Rust port [ready]

## Decisions
- Left #2 blocked because the workspace still lacks Buildkite org credentials or runner access; requested the necessary details from the human team.
- Created #5 and #6 to keep 2+ ready issues available, focusing on the worker lifecycle and scheduler specs needed before deeper Rust rewrites.

## Blockers
- #2 cannot proceed until Buildkite org/agent credentials and runner access are provided.

## Next Priorities
1. Finish #4 to establish the Rust crate and FFI hook.
2. Provide the Buildkite credentials needed to unblock #2.
3. Complete the specification work in #5 and #6 so subsequent coding tasks have clear boundaries.
