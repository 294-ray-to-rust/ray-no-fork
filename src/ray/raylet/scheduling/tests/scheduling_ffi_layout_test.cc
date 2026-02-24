#include <stddef.h>

#include "gtest/gtest.h"
#include "ray/raylet/scheduling/ffi/scheduling_ffi.h"

namespace ray::raylet::ffi {

static_assert(sizeof(void *) == 8, "Scheduling FFI assumes 64-bit ABI.");
static_assert(sizeof(RayletStr) == 16, "RayletStr should be pointer+len.");
static_assert(sizeof(RayletStrArray) == 16, "RayletStrArray should be pointer+len.");
static_assert(sizeof(RayletResourceEntry) == 24, "RayletResourceEntry layout changed.");
static_assert(sizeof(RayletLabelEntry) == 32, "RayletLabelEntry layout changed.");
static_assert(offsetof(RayletResourceEntry, value) == sizeof(RayletStr),
              "RayletResourceEntry value offset mismatch.");
static_assert(offsetof(RayletLabelEntry, value) == sizeof(RayletStr),
              "RayletLabelEntry value offset mismatch.");
static_assert(sizeof(RayletPgBundleSpec) == 48, "RayletPgBundleSpec layout changed.");
static_assert(sizeof(RayletPgBundleAllocation) == 56,
              "RayletPgBundleAllocation layout changed.");
static_assert(sizeof(RayletPgCommitReleaseResult) == 40,
              "RayletPgCommitReleaseResult layout changed.");
static_assert(offsetof(RayletPgBundleSpec, required_resources) == 32,
              "RayletPgBundleSpec required_resources offset mismatch.");
static_assert(offsetof(RayletPgBundleAllocation, allocated_resources) == 40,
              "RayletPgBundleAllocation allocated_resources offset mismatch.");
static_assert(offsetof(RayletPgCommitReleaseResult, operation) == 32,
              "RayletPgCommitReleaseResult operation offset mismatch.");
static_assert(sizeof(RayletPgCommitReleaseOp) == 1,
              "RayletPgCommitReleaseOp ABI width changed.");
static_assert(sizeof(RayletPgResultCode) == 1, "RayletPgResultCode ABI width changed.");

TEST(SchedulingFfiLayoutTest, RequestDecisionSizes) {
  EXPECT_EQ(sizeof(RayletResourceRequest), 40u);
  EXPECT_EQ(sizeof(RayletSchedulingRequest), 56u);
  EXPECT_EQ(sizeof(RayletSchedulingDecision), 24u);
}

TEST(SchedulingFfiLayoutTest, PlacementGroupReservationTypes) {
  EXPECT_EQ(kRayletPgAbiVersion, 1u);
  EXPECT_EQ(sizeof(RayletPgBundleSpec), 48u);
  EXPECT_EQ(sizeof(RayletPgBundleAllocation), 56u);
  EXPECT_EQ(sizeof(RayletPgCommitReleaseResult), 40u);
}

}  // namespace ray::raylet::ffi
