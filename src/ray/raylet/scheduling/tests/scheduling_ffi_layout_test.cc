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

TEST(SchedulingFfiLayoutTest, RequestDecisionSizes) {
  EXPECT_EQ(sizeof(RayletResourceRequest), 40u);
  EXPECT_EQ(sizeof(RayletSchedulingRequest), 56u);
  EXPECT_EQ(sizeof(RayletSchedulingDecision), 24u);
}

}  // namespace ray::raylet::ffi
