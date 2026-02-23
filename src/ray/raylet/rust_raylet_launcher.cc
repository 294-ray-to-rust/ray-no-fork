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

#include "ray/util/env.h"
#include "ray/util/logging.h"

extern "C" int raylet_entrypoint();

namespace ray::raylet {
namespace {
constexpr char kRayletUseRustEnvVar[] = "RAYLET_USE_RUST";
}  // namespace

bool ShouldUseRustRaylet() { return ray::IsEnvTrue(kRayletUseRustEnvVar); }

absl::optional<int> RunRustRayletIfEnabled() {
  if (!ShouldUseRustRaylet()) {
    return absl::nullopt;
  }

  RAY_LOG(INFO) << "RAYLET_USE_RUST is set, invoking Rust raylet entrypoint.";
  const int status = raylet_entrypoint();
  if (status != 0) {
    RAY_LOG(ERROR) << "Rust raylet entrypoint failed with status " << status;
  }
  return status;
}

}  // namespace ray::raylet
