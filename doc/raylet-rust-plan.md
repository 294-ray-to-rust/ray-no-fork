# Raylet Rust Migration Plan

## Current Raylet Subsystems

| Subsystem | Key files | Responsibilities | Notes |
| --- | --- | --- | --- |
| Node lifecycle & RPC frontdoor | `src/ray/raylet/main.cc`, `src/ray/rpc/node_manager/node_manager_server.*`, `src/ray/raylet/node_manager.{cc,h}` | Owns process startup, flag parsing, bootstrap of `NodeManager`, serves gRPC endpoints for worker leases, drains, task cancellation, and RaySyncer control | Fans in most other subsystems through dependency injection, orchestrates GCS subscriptions and RaySyncer heartbeat handling |
| Scheduling & resource management | `src/ray/raylet/scheduling/*`, `src/ray/raylet/placement_group_resource_manager.*`, `src/ray/raylet/lease_dependency_manager.*`, `src/ray/raylet/wait_manager.*` | Maintains cluster + local resource state, allocates leases, tracks PGS bundles, enforces fairness via `ClusterResourceScheduler` and `LocalLeaseManager`, fulfills `ray.get/wait` semantics | Tight loop with worker pool and RaySyncer; uses proto types defined under `src/ray/protobuf` for scheduling data |
| Worker lifecycle & runtime env | `src/ray/raylet/worker_pool.*`, `src/ray/raylet/worker.{cc,h}`, `src/ray/raylet/runtime_env_agent_client.*`, `src/ray/raylet/agent_manager.*`, `src/ray/raylet/worker_killing_policy_*` | Spawns, registers, tracks, and terminates workers/drivers; integrates runtime env agent + dashboard agent; enforces worker killing heuristics | Tied to process + cgroup hooks defined in `main.cc`; manipulates shared worker RPC pool |
| Local object & spill management | `src/ray/raylet/local_object_manager.{cc,h}`, `src/ray/raylet/local_object_manager_interface.h`, `src/ray/raylet/wait_manager.*`, `src/ray/raylet/throttler.h` | Pins objects requested by workers, orchestrates spilling/restore via IO workers, tracks wait requests when `fetch_local=true`, throttles plasma interactions | Collaborates with `ray/object_manager/*`, `pubsub` subscriber, and IO worker pool APIs |
| Cluster comms & heartbeats | `src/ray/ray_syncer/*`, `src/ray/raylet/metrics.h`, `src/ray/raylet/agent_manager.*`, `src/ray/raylet/runtime_env_agent_client.*` | Broadcasts local resource usage, receives cluster resource view changes, reports metrics, coordinates GC commands, starts ancillary agents | RaySyncer + metrics surfaces need deterministic cadence to satisfy control-plane SLAs |
| Object manager hooks | `src/ray/object_manager/*`, `src/ray/raylet/local_object_manager_interface.h`, `src/ray/raylet/local_object_manager.{cc,h}`, `src/ray/raylet/node_manager.cc` | NodeManager responds to `HandleObjectLocal/Missing`, updates object directory, drives `PinObjectIDs`, `AsyncRestoreSpilledObject`, and spill delete queue | Boundary between plasma/object_manager threads and raylet single-threaded state machine |
| IPC / RPC clients | `src/ray/raylet_rpc_client/*`, `src/ray/raylet_ipc_client/*`, `python/ray/_raylet.{pyx,pxd,pyi}` | Provide client shims for worker + Python integration, enabling `core_worker` to talk to raylet via gRPC or UNIX domain sockets | Any Rust replacement must preserve ABI for Python extension `_raylet` until Py side moves |
| Supporting infra | `src/ray/raylet/metrics.h`, `src/ray/raylet/agent_manager.*`, `src/ray/raylet/worker_pool.*`, `src/ray/raylet/throttler.h`, `src/ray/raylet/main.cc` | Metrics emission, throttling policies, process supervision, signal handling, cgroup wiring | These pieces can migrate gradually once core scheduling + worker lifecycle live in Rust |

## Proposed Migration Phases

Each phase is scoped so it can land within a single SWE iteration while leaving C++ fallbacks in place via FFI shims.

### Phase 1 – Scheduler shim in Rust
- Build `raylet_rs` crate with `ClusterResourceScheduler` + `LocalResourceManager` equivalents.
- Provide FFI so existing `NodeManager::ScheduleTasks` calls into Rust for placement decisions while everything else stays in C++.
- Deliver deterministic parity on scheduling unit tests (`src/ray/raylet/scheduling/tests`).

### Phase 2 – Worker lease pipeline
- Port `WorkerPool`, `Worker`, and worker-start throttling logic to Rust, still invoked by C++ `NodeManager`.
- Rust side issues callbacks back into C++ for process launching + runtime env agent RPCs until those move as well.

### Phase 3 – Placement group + bundle resource tracking
- Re-implement `PlacementGroupResourceManager` + `LeaseDependencyManager` in Rust on top of the Phase 1 scheduler state.
- Preserve existing two-phase commit semantics for bundles and add integration tests for PG scheduling.

### Phase 4 – Local object + wait management
- Port `LocalObjectManager`, spill/restore state machines, and `WaitManager` to Rust.
- Maintain FFI adapters for IO worker pool + plasma callbacks while plasma remains C++.

### Phase 5 – GCS/heartbeat + agent coordination
- Replace C++ `NodeManager` control loop with Rust struct that owns scheduler, worker pool, placement + object managers.
- Move RaySyncer participation, metrics emission, and agent supervision into Rust, leaving `main.cc` as a thin bootstrap shim.

### Phase 6 – Python/module integration cleanup
- Swap `_raylet` bindings to target Rust implementation (e.g., via `PyO3` or `cxx` generated C API) and retire C++ source files.
- Remove transitional shims and delete defunct `src/ray/raylet/*.cc` once parity verified.

## Phase 1 FFI Boundary

**Goal:** call the Rust scheduler core from the existing C++ `NodeManager` without reworking worker management yet.

### Functions
1. `extern "C" RayletSchedulerHandle *raylet_rs_scheduler_create(const RayletSchedulerConfig *config);`
   - Config mirrors `ClusterResourceScheduler` ctor inputs (local resources, scheduling config flags, placement group enablement).
2. `extern "C" void raylet_rs_scheduler_destroy(RayletSchedulerHandle *handle);`
3. `extern "C" void raylet_rs_scheduler_update_cluster_view(RayletSchedulerHandle *, const SchedulerResourceUpdate *update_batch);`
   - Called from `NodeManager::UpdateResourceUsage` whenever GCS sends cluster resource deltas.
4. `extern "C" bool raylet_rs_scheduler_allocate(RayletSchedulerHandle *, const SchedulingRequest *request, SchedulingDecision *out);`
   - Consumes lease spec (resources, placement constraints, failure info) and produces a decision (node id, resource instances, spillback target).
5. `extern "C" void raylet_rs_scheduler_release(RayletSchedulerHandle *, const SchedulingRelease *release);`
   - Releases resources when tasks finish or are canceled.

### Data Types
- `RayletSchedulerConfig`: POD struct (C ABI) containing copies of `NodeManagerConfig::resource_config`, scheduling tunables, and pointer-sized handles for callbacks (e.g., logging, stats) so Rust can report metrics without touching glog directly.
- `SchedulingRequest`: flattened version of `rpc::RequestWorkerLeaseRequest`, keeping IDs (JobID, SchedulingClass, PlacementGroupID), required resources, and spillback hints.
- `SchedulingDecision`: contains allocation success flag, selected node, resource instances (`FixedPoint` arrays converted to doubles), and metadata whether to trigger worker start locally.
- `SchedulerResourceUpdate`: batched updates from GCS containing node id, available/total resources, draining status; matches `rpc::ResourceUsageBatchData` fields but serialized into a simple array for zero-copy FFI.

### ABI-safe scheduling data model

The shared ABI structs live in `src/ray/raylet/scheduling/ffi/scheduling_ffi.h` and
`rust/raylet-rs/src/scheduling_ffi.rs`.

- `RayletStr` + `RayletStrArray` hold UTF-8 string slices for resource names and label keys.
- `RayletResourceEntry` + `RayletResourceArray` encode resource vectors as name/value pairs.
- `RayletLabelConstraint`/`RayletLabelSelector` capture label selector requirements for
  scheduling requests.
- `RayletNodeResources` captures total/available/load resources plus draining metadata.
- `RayletNodeResourceViewArray` batches per-node resource snapshots for updates.
- `RayletSchedulingRequest` carries a resource request and preferred node hint.
- `RayletSchedulingDecision` returns the selected node and feasibility flags.

### ABI evolution guidelines

- Treat all structs as `#[repr(C)]`/POD and only append fields at the end.
- Use new structs or versioned variants instead of reordering or repurposing fields.
- Keep enums `repr(u8)` and avoid widening without a new type.
- Preserve pointer+length ownership contracts; Rust never frees memory it didn't allocate.
- Add layout tests on both sides when fields change.

### Data Flow
1. C++ `NodeManager` builds a `RayletSchedulerConfig` at startup and calls `raylet_rs_scheduler_create`, keeping the opaque handle.
2. On every GCS resource view update, `NodeManager::HandleResourceUsageBatch` converts `rpc::ResourceUsageBatchData` to `SchedulerResourceUpdate` and calls `raylet_rs_scheduler_update_cluster_view`.
3. When `HandleRequestWorkerLease` executes, it marshals the request into `SchedulingRequest` and calls `raylet_rs_scheduler_allocate`.
4. Rust scheduler returns a `SchedulingDecision` telling C++ whether to run locally, spillback, or mark infeasible. C++ proceeds exactly as before (assign worker, push to dispatch queue, etc.).
5. When the lease completes or fails, `NodeManager` informs Rust through `raylet_rs_scheduler_release` so resources become available again.

### Tooling + Tests
- Use the `cxx` crate so Rust can own the scheduler types yet expose a stable C++ header living under `src/ray/raylet/scheduling/rust_scheduler_ffi.h`.
- Mirror existing C++ scheduler unit tests in Rust (`raylet_rs/tests/scheduler_tests.rs`) and keep the legacy tests compiling to detect regressions while the new crate lands.

### Scheduler FFI scaffolding location
- Rust bridge types/functions live in `rust/raylet-rs/src/scheduler_ffi.rs`; add new ABI structs and extern functions there.
- The generated header is `src/ray/raylet/scheduling/rust_scheduler_ffi.h` and is exported to C++ via `//rust/raylet-rs:raylet_rs_scheduler_ffi`.
- C++ validation/smoke tests should live under `src/ray/raylet/scheduling/tests`.

## Next Steps
- Create an issue per phase starting with the scheduler shim, referencing this document.
- Align with build owners to introduce a `rust/` workspace section (Cargo + Bazel target) for the new crate, ensuring CI builds `raylet_rs` shared library alongside existing binaries.
