#!/usr/bin/env bash

# This script generate a ray C++ template and run example
set -e
rm -rf ray-template
ray cpp --generate-bazel-project-template-to ray-template
(
    cd ray-template

    # Keep the template build in sync with its checked-in .bazelversion.
    USE_BAZEL_VERSION="$(cat .bazelversion)" bash run.sh
)
