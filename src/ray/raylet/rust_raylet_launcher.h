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

#pragma once

#include "absl/types/optional.h"

namespace ray::raylet {

bool ShouldUseRustRaylet();

/// Runs the Rust raylet entrypoint if the feature flag is enabled.
/// Returns std::nullopt when the legacy C++ path should be used.
absl::optional<int> RunRustRayletIfEnabled();

}  // namespace ray::raylet
