# syntax=docker/dockerfile:1.3-labs
ARG ARCH_SUFFIX=
ARG HOSTTYPE=x86_64
ARG MANYLINUX_VERSION
FROM rayrust.ej2.org:5010/dockerhub/rayproject/manylinux2014:${MANYLINUX_VERSION}-jdk-${HOSTTYPE} AS builder

ARG PYTHON_VERSION=3.10
ARG BUILDKITE_BAZEL_CACHE_URL
ARG BUILDKITE_CACHE_READONLY
ARG HOSTTYPE
ARG IS_LOCAL_BUILD=false
ARG CACHE_DIR=/opt/cache

ENV BUILDKITE_BAZEL_CACHE_URL=${BUILDKITE_BAZEL_CACHE_URL}
ENV BUILDKITE_CACHE_READONLY=${BUILDKITE_CACHE_READONLY}
ENV IS_LOCAL_BUILD=${IS_LOCAL_BUILD}
ENV CACHE_DIR=${CACHE_DIR}
ENV DOWNLOAD_CACHE=${CACHE_DIR}/downloads
ENV BAZEL_CACHE=${CACHE_DIR}/bazel

WORKDIR /home/forge/ray

COPY . .

RUN --mount=type=cache,target=${DOWNLOAD_CACHE},uid=2000,gid=100,id=ray-downloads-${HOSTTYPE} \
    --mount=type=cache,target=${BAZEL_CACHE},uid=2000,gid=100,id=ray-bazel-${HOSTTYPE}-py${PYTHON_VERSION} \
    <<'EOFRUN'
#!/bin/bash
set -euo pipefail

# Set up compiler toolchain (manylinux2014 devtoolset)
# Disable unbound variable check - devtoolset scripts use unset vars
set +u
if [[ -d /opt/rh/devtoolset-10 ]]; then
    source /opt/rh/devtoolset-10/enable
elif [[ -d /opt/rh/devtoolset-8 ]]; then
    source /opt/rh/devtoolset-8/enable
fi
set -u

# Install Rust toolchain for building the Rust backend.
#
# RUSTUP_HOME and CARGO_HOME live in the shared ray-downloads-${HOSTTYPE}
# BuildKit cache mount, which is used concurrently by all PYTHON_VERSION
# variants of this image (py3.10..py3.14) when wanda builds them in
# parallel. Two failure modes have been observed:
#
#   1. The old `[[ ! -f cargo ]]` guard is satisfied after rustup-init
#      succeeds, but the *real* toolchain install happens implicitly
#      when `cargo build` reads rust/rust-toolchain.toml (channel=stable,
#      components=[rustfmt, clippy]). Parallel builds race while rustup
#      unpacks components into $RUSTUP_HOME and one loses with:
#
#        error: failed to install component: 'clippy-preview-...':
#        detected conflict: 'lib/rustlib/manifest-clippy-preview-...'
#
#      The conflicting manifest persists in the cache and every
#      subsequent build hits the same error on recovery.
#
#   2. The guard also can't tell a partially-installed toolchain from a
#      healthy one, so it never self-heals.
#
# Fix: serialize rustup init + toolchain install with a file lock held
# on a lockfile inside the shared cache, and probe cargo functionally
# (not just its presence on disk). The first concurrent build populates
# the cache; the rest see a fast no-op cache hit.
export RUSTUP_HOME=$DOWNLOAD_CACHE/rustup
export CARGO_HOME=$DOWNLOAD_CACHE/cargo
export PATH="$CARGO_HOME/bin:$PATH"

mkdir -p "$DOWNLOAD_CACHE"
(
    flock 9

    if ! command -v cargo >/dev/null 2>&1 || ! cargo --version >/dev/null 2>&1; then
        rm -rf "$RUSTUP_HOME" "$CARGO_HOME"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
            -y --default-toolchain 1.85.0 --no-modify-path --profile minimal
    fi

    # Ensure the toolchain pinned by rust/rust-toolchain.toml (stable +
    # rustfmt + clippy) is fully installed. Probe the actual binaries
    # in $RUSTUP_HOME — `rustup show` reports a partial install as OK,
    # which is what let the race in #333 corrupt the cache persistently.
    #
    # If any of the binaries is missing or unrunnable we're looking at
    # either a cold cache or a leftover partial install from a previous
    # failed concurrent build (manifest-*-preview files left behind).
    # Wipe the toolchains dir so rustup reinstalls cleanly.
    tc_dir="$RUSTUP_HOME/toolchains/stable-${HOSTTYPE}-unknown-linux-gnu"
    toolchain_ok=1
    for bin in "$tc_dir/bin/rustc" "$tc_dir/bin/cargo-clippy" "$tc_dir/bin/rustfmt"; do
        if [[ ! -x "$bin" ]] || ! "$bin" --version >/dev/null 2>&1; then
            toolchain_ok=0
            break
        fi
    done
    if [[ $toolchain_ok -eq 0 ]]; then
        rm -rf "$RUSTUP_HOME/toolchains"
        rustup toolchain install stable \
            --profile minimal --component rustfmt --component clippy
    fi
) 9>"$DOWNLOAD_CACHE/.rustup.lock"

# Install protoc (required by prost-build/tonic-build for ray-proto codegen).
# Cached in $DOWNLOAD_CACHE so it is only downloaded once across all builds.
PROTOC_VERSION=28.3
PROTOC_BIN="$DOWNLOAD_CACHE/protoc/bin/protoc"
if [[ ! -x "$PROTOC_BIN" ]]; then
    mkdir -p "$DOWNLOAD_CACHE/protoc"
    curl -sSfL "https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-x86_64.zip" \
        -o /tmp/protoc.zip
    unzip -q -o /tmp/protoc.zip -d "$DOWNLOAD_CACHE/protoc"
fi
export PATH="$DOWNLOAD_CACHE/protoc/bin:$PATH"
export PROTOC="$DOWNLOAD_CACHE/protoc/bin/protoc"

# Build the Rust backend BEFORE running Bazel
# This creates rust/_raylet.so which Bazel will package.
#
# The RUN step executes as uid 2000 (forge) to match the cache mount
# ownership, but `COPY . .` leaves the source tree owned by root, so
# `rust/` is not writable by the build user. Point cargo at a target
# dir under $DOWNLOAD_CACHE (uid 2000) so it can create its build dir.
echo "Building Rust backend..."
export CARGO_TARGET_DIR=$DOWNLOAD_CACHE/cargo-target-py${PYTHON_VERSION}
mkdir -p "$CARGO_TARGET_DIR"
cd rust
cargo build --release --package ray-core-worker-pylib --features python
cp "$CARGO_TARGET_DIR/release/lib_raylet.so" _raylet.so
cd ..
echo "Rust backend built: rust/_raylet.so"

export BAZELISK_HOME=$DOWNLOAD_CACHE/bazelisk
REPOSITORY_CACHE=$DOWNLOAD_CACHE/repo

PY_CODE="${PYTHON_VERSION//./}"
PY_BIN="cp${PY_CODE}-cp${PY_CODE}"

export RAY_BUILD_ENV="manylinux_py${PY_BIN}"

sudo ln -sf "/opt/python/${PY_BIN}/bin/python3" /usr/local/bin/python3
sudo ln -sf /usr/local/bin/python3 /usr/local/bin/python

BAZEL_CACHE_ARGS=""
if [[ -z "${BUILDKITE_BAZEL_CACHE_URL:-}" ]]; then
  # Disable remote cache for local builds (no credentials)
  BAZEL_CACHE_ARGS="--remote_cache="
elif [[ "${BUILDKITE_CACHE_READONLY:-}" == "true" ]]; then
  # Read-only mode: disable uploads only
  BAZEL_CACHE_ARGS="--remote_upload_local_results=false"
else
  # Override any remote_cache baked into the base image's ~/.bazelrc
  # (upstream manylinux images have the S3 URL which we can't access).
  BAZEL_CACHE_ARGS="--remote_cache=${BUILDKITE_BAZEL_CACHE_URL}"
fi

BAZEL_RESOURCE_FLAGS=""
if [[ "$IS_LOCAL_BUILD" == "true" ]]; then
  BAZEL_RESOURCE_FLAGS=$(python3 "$HOME/ray/ci/build/container_resource_utils.py")
fi

bazelisk --output_base=$BAZEL_CACHE build --config=ci \
    --repository_cache=$REPOSITORY_CACHE \
    $BAZEL_CACHE_ARGS \
    $BAZEL_RESOURCE_FLAGS \
    //:ray_pkg_zip //:ray_py_proto_zip

cp bazel-bin/ray_pkg.zip /home/forge/ray_pkg.zip
cp bazel-bin/ray_py_proto.zip /home/forge/ray_py_proto.zip

EOFRUN

FROM scratch

COPY --from=builder /home/forge/ray_pkg.zip /home/forge/ray_py_proto.zip /
