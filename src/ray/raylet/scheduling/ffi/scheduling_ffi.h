#pragma once

#include <stddef.h>
#include <stdint.h>

namespace ray::raylet::ffi {

struct RayletStr {
  // Pointer+length UTF-8 view owned by the caller.
  // Rust reads the slice and never takes ownership.
  const char *data;
  size_t len;
};

struct RayletStrArray {
  const RayletStr *entries;
  size_t len;
};

struct RayletByteArray {
  // Pointer+length byte view owned by the caller.
  // Rust reads the slice and never takes ownership.
  const uint8_t *data;
  size_t len;
};

enum class RayletWorkerType : uint8_t {
  kWorker = 0,
  kDriver = 1,
  kSpillWorker = 2,
  kRestoreWorker = 3,
};

enum class RayletLanguage : uint8_t {
  kPython = 0,
  kJava = 1,
  kCpp = 2,
  kRust = 3,
};

enum class RayletWorkerReleaseReason : uint8_t {
  kTaskFinished = 0,
  kTaskCanceled = 1,
  kPreempted = 2,
  kDisconnected = 3,
};

enum class RayletWorkerExitType : uint8_t {
  kIntended = 0,
  kSystemError = 1,
  kUserError = 2,
  kNodeShutdown = 3,
};

struct RayletResourceEntry {
  RayletStr name;
  double value;
};

struct RayletResourceArray {
  const RayletResourceEntry *entries;
  size_t len;
};

struct RayletWorkerIdentity {
  RayletByteArray worker_id;
  RayletByteArray job_id;
  RayletByteArray actor_id;
  RayletByteArray node_id;
  RayletWorkerType worker_type;
  RayletLanguage language;
  uint8_t reserved0[6];
};

struct RayletWorkerState {
  RayletWorkerIdentity identity;
  int32_t process_id;
  int32_t worker_port;
  int64_t startup_token;
  uint8_t is_registered;
  uint8_t is_idle;
  uint8_t is_detached_actor;
  uint8_t reserved0[5];
};

struct RayletWorkerRegisterRequest {
  RayletWorkerState state;
  RayletStr worker_address;
  RayletByteArray serialized_runtime_env;
  int32_t debugger_port;
  uint8_t reserved0[4];
};

struct RayletWorkerLeaseRequest {
  int64_t lease_id;
  RayletByteArray worker_id;
  int64_t scheduling_class;
  RayletResourceArray required_resources;
  RayletResourceArray placement_resources;
  uint8_t is_actor_creation_task;
  uint8_t grant_or_reject;
  uint8_t reserved0[6];
};

struct RayletWorkerReleaseRequest {
  int64_t lease_id;
  RayletByteArray worker_id;
  RayletWorkerReleaseReason release_reason;
  uint8_t return_worker_to_idle;
  uint8_t worker_exiting;
  uint8_t reserved0[5];
};

struct RayletWorkerExitEvent {
  RayletByteArray worker_id;
  RayletWorkerType worker_type;
  RayletWorkerExitType exit_type;
  uint8_t has_creation_task_exception;
  uint8_t reserved0[5];
  RayletStr exit_detail;
  int32_t exit_code;
  uint8_t reserved1[4];
};

struct RayletLabelEntry {
  RayletStr key;
  RayletStr value;
};

struct RayletLabelArray {
  const RayletLabelEntry *entries;
  size_t len;
};

enum class RayletLabelSelectorOp : uint8_t {
  kUnspecified = 0,
  kIn = 1,
  kNotIn = 2,
};

struct RayletLabelConstraint {
  RayletStr key;
  RayletLabelSelectorOp op;
  RayletStrArray values;
};

struct RayletLabelSelector {
  const RayletLabelConstraint *constraints;
  size_t len;
};

struct RayletResourceRequest {
  RayletResourceArray resources;
  uint8_t requires_object_store_memory;
  RayletLabelSelector label_selector;
};

struct RayletNodeResources {
  RayletResourceArray total;
  RayletResourceArray available;
  RayletResourceArray load;
  RayletResourceArray normal_task_resources;
  RayletLabelArray labels;
  int64_t idle_resource_duration_ms;
  uint8_t is_draining;
  int64_t draining_deadline_timestamp_ms;
  int64_t last_resource_update_ms;
  int64_t latest_resources_normal_task_timestamp;
  uint8_t object_pulls_queued;
};

struct RayletNodeResourceView {
  int64_t node_id;
  RayletNodeResources resources;
};

struct RayletNodeResourceViewArray {
  const RayletNodeResourceView *entries;
  size_t len;
};

struct RayletSchedulingRequest {
  int64_t request_id;
  int64_t preferred_node_id;
  RayletResourceRequest resource_request;
};

struct RayletSchedulingDecision {
  int64_t request_id;
  int64_t selected_node_id;
  uint8_t is_feasible;
  uint8_t is_spillback;
};

inline RayletStr RayletStrFromRaw(const char *data, size_t len) {
  return RayletStr{data, len};
}

inline RayletResourceArray RayletResourceArrayFromRaw(const RayletResourceEntry *entries,
                                                      size_t len) {
  return RayletResourceArray{entries, len};
}

inline RayletLabelArray RayletLabelArrayFromRaw(const RayletLabelEntry *entries,
                                                size_t len) {
  return RayletLabelArray{entries, len};
}

inline RayletLabelSelector RayletLabelSelectorFromRaw(
    const RayletLabelConstraint *entries, size_t len) {
  return RayletLabelSelector{entries, len};
}

inline RayletStrArray RayletStrArrayFromRaw(const RayletStr *entries, size_t len) {
  return RayletStrArray{entries, len};
}

inline RayletByteArray RayletByteArrayFromRaw(const uint8_t *data, size_t len) {
  return RayletByteArray{data, len};
}

extern "C" {
uint8_t raylet_rs_scheduler_roundtrip(const RayletSchedulingRequest *request,
                                      RayletSchedulingDecision *decision_out);
}

}  // namespace ray::raylet::ffi
