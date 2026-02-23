# raylet-rs scheduling status

## ClusterResourceScheduler port

- The core node/resource tracking and request allocation loop now lives in Rust in
  `src/cluster_resource_scheduler.rs`.
- FFI wrappers for scheduler lifecycle, update, allocate, and release are exported from
  `src/scheduling_ffi.rs` and declared in
  `src/ray/raylet/scheduling/ffi/scheduling_ffi.h`.
- `ClusterResourceScheduler` in C++ can delegate the default scheduling path to Rust when
  `RAYLET_USE_RUST=1`.

## Known deviations and TODOs

- The Rust delegation path currently only handles the default scheduling strategy branch;
  spread, node-affinity, placement-group, and other policy-specific paths still use C++.
- CMake wiring is not present in this workspace; Bazel linking is in place and matches the
  current project build system usage.
