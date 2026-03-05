# ci/docker/cpp-ci.Dockerfile
#
# Self-contained CI image for building and testing Ray's C++ code with Bazel.
# Combines the relevant pieces from base.test.Dockerfile and base.build.Dockerfile
# into a single stage, omitting Python/Java test dependencies we don't need.
#
# Reference images:
#   - ci/docker/base.test.Dockerfile  (system packages, clang, bazel)
#   - ci/docker/base.build.Dockerfile (CC/CXX env vars)

FROM ubuntu:focal

# ---------- build args ----------
ARG BUILDKITE_BAZEL_CACHE_URL

# ---------- environment ----------
# Prevent apt prompts during build
ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=America/Los_Angeles

# Compiler flags — match base.build.Dockerfile
ENV CC=clang
ENV CXX=clang++-12

# CI flags expected by install-bazel.sh and Bazel configs
ENV BUILDKITE=true
ENV CI=true
ENV BUILDKITE_BAZEL_CACHE_URL=${BUILDKITE_BAZEL_CACHE_URL}

# ---------- system packages ----------
# Mirrors base.test.Dockerfile — only the packages needed for C++ builds.
RUN apt-get update -qq && apt-get upgrade -qq -y \
    && apt-get install -y -qq --no-install-recommends \
        build-essential \
        curl \
        git \
        zip \
        unzip \
        wget \
        sudo \
        # Clang 12 toolchain
        clang-12 \
        clang-format-12 \
        clang-tidy-12 \
        # Build dependencies
        cmake \
        zlib1g-dev \
        liblz4-dev \
        libunwind-dev \
        libncurses5 \
        # OpenSSL build (rules_foreign_cc) requires perl for ./Configure
        perl \
        # rules_foreign_cc build tool dependencies
        pkg-config \
        # Python (needed by Bazel's Python toolchain rules)
        python-is-python3 \
        python3 \
        python3-pip \
        # deadsnakes PPA prerequisites
        software-properties-common \
    && rm -rf /var/lib/apt/lists/*

# ---------- clang symlinks ----------
# So that CC=clang / clang-format / clang-tidy resolve without version suffix.
RUN ln -s /usr/bin/clang-12 /usr/bin/clang \
    && ln -s /usr/bin/clang-format-12 /usr/bin/clang-format \
    && ln -s /usr/bin/clang-tidy-12 /usr/bin/clang-tidy

# ---------- Python 3.10 ----------
# Bazel's Python toolchain rules require Python 3.10.
# We install it from the deadsnakes PPA, which is simpler and more
# reproducible in a Dockerfile than the conda-based miniforge approach.
RUN add-apt-repository ppa:deadsnakes/ppa \
    && apt-get update -qq \
    && apt-get install -y -qq --no-install-recommends \
        python3.10 \
        python3.10-dev \
        python3.10-distutils \
        python3.10-venv \
    && rm -rf /var/lib/apt/lists/* \
    # Make python3.10 the default python3 / python
    && update-alternatives --install /usr/bin/python3 python3 /usr/bin/python3.10 1 \
    && update-alternatives --install /usr/bin/python python /usr/bin/python3.10 1

# ---------- Bazel (via bazelisk) ----------
# Matches ci/env/install-bazel.sh — installs bazelisk v1.16.0 as /bin/bazel.
ARG BAZELISK_VERSION=v1.16.0
RUN ARCH=$(dpkg --print-architecture) \
    && curl -fsSL -o /bin/bazel \
       "https://github.com/bazelbuild/bazelisk/releases/download/${BAZELISK_VERSION}/bazelisk-linux-${ARCH}" \
    && chmod +x /bin/bazel \
    # Verify installation
    && bazel --version

# ---------- source tree ----------
RUN mkdir -p /ray
WORKDIR /ray
COPY . .
