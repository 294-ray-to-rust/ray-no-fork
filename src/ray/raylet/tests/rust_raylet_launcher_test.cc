// Copyright 2026 The Ray Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#include "ray/raylet/rust_raylet_launcher.h"

#include "gtest/gtest.h"
#include "ray/util/env.h"

namespace ray::raylet {
namespace {
constexpr char kRayletUseRustEnvVar[] = "RAYLET_USE_RUST";
}  // namespace

TEST(RustRayletLauncherTest, RunsWhenEnabled) {
  ray::SetEnv(kRayletUseRustEnvVar, "1");
  const auto status = RunRustRayletIfEnabled();
  ASSERT_TRUE(status.has_value());
  EXPECT_EQ(*status, 0);
  ray::UnsetEnv(kRayletUseRustEnvVar);
}

TEST(RustRayletLauncherTest, SkipsWhenDisabled) {
  ray::UnsetEnv(kRayletUseRustEnvVar);
  const auto status = RunRustRayletIfEnabled();
  EXPECT_FALSE(status.has_value());
}

}  // namespace ray::raylet
