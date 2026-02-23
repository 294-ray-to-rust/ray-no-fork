#pragma once

#include <stddef.h>
#include <stdint.h>

namespace ray::raylet::ffi {

struct RayletStr {
  const char *data;
  size_t len;
};

struct RayletStrArray {
  const RayletStr *entries;
  size_t len;
};

struct RayletResourceEntry {
  RayletStr name;
  double value;
};

struct RayletResourceArray {
  const RayletResourceEntry *entries;
  size_t len;
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

struct RayletClusterResourceScheduler;

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

extern "C" {
RayletClusterResourceScheduler *raylet_rs_cluster_resource_scheduler_new();
void raylet_rs_cluster_resource_scheduler_free(RayletClusterResourceScheduler *scheduler);
uint8_t raylet_rs_cluster_resource_scheduler_update(
    RayletClusterResourceScheduler *scheduler, const RayletNodeResourceViewArray *nodes);
uint8_t raylet_rs_cluster_resource_scheduler_allocate(
    RayletClusterResourceScheduler *scheduler,
    const RayletSchedulingRequest *request,
    RayletSchedulingDecision *decision_out);
uint8_t raylet_rs_cluster_resource_scheduler_release(
    RayletClusterResourceScheduler *scheduler,
    int64_t node_id,
    const RayletResourceArray *resources);
uint8_t raylet_rs_scheduler_roundtrip(const RayletSchedulingRequest *request,
                                      RayletSchedulingDecision *decision_out);
}

}  // namespace ray::raylet::ffi
