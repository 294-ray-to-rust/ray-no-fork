# Project Memory

## Status
- **Project status**: in_progress
- **Last run**: 2026-02-23T18:28:29Z
- **Run count**: 2

## Summary
Initial scaffolding is complete (Rust crate plus migration plan). Focus now shifts to bridging the C++ entrypoint into Rust and beginning the scheduling subsystem port.

## Completed Issues
- #1: Set up Rust raylet crate
- #2: Raylet Rust rewrite plan

## Active Issues
- #5: Bridge C++ raylet main to Rust FFI [ready]
- #6: Port LocalResourceManager logic to Rust [ready]
- #7: Define ABI-safe raylet scheduling data model [ready]

## Decisions
- Closed out planning/scaffolding work (#1, #2) since the SWE agent completed them.
- Created #5 to ensure the new Rust crate is actually exercised by the C++ entrypoint.
- Created #6 to move the first concrete scheduling component into Rust with FFI coverage.
- Created #7 to formalize the ABI contract between C++ and Rust so later ports share a stable data model.

## Blockers
None.

## Next Priorities
1. Implement #5 to connect the runtime to the Rust entrypoint.
2. Land #6 so the LocalResourceManager is powered by Rust.
3. Complete #7 to lock down the cross-language ABI for scheduling data.
