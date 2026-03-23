ARG DOCKER_IMAGE_BASE_TEST=cr.ray.io/rayproject/oss-ci-base_test
FROM $DOCKER_IMAGE_BASE_TEST

ARG APT_PROXY=""
RUN if [ -n "$APT_PROXY" ]; then \
      echo "Acquire::http::Proxy \"$APT_PROXY\";" > /etc/apt/apt.conf.d/01proxy; \
    fi

ARG PIP_INDEX_URL=""
ARG PIP_TRUSTED_HOST=""

ARG RAYCI_DISABLE_JAVA=false

# Stage 1: dependency specs from wanda srcs (rarely change).
# Files not in wanda srcs (e.g. .bazelversion) persist from the base
# image, so we only COPY what's explicitly in the build context.
COPY ci/ ci/
COPY .bazelrc .bazelrc
COPY python/requirements.txt python/requirements.txt
COPY python/requirements_compiled.txt python/requirements_compiled.txt
COPY python/requirements/test-requirements.txt python/requirements/test-requirements.txt

RUN <<EOF
#!/bin/bash -i
set -euo pipefail
if [[ "$RAYCI_DISABLE_JAVA" != "true" ]]; then
    apt-get update -y
    apt-get install -y -qq maven openjdk-8-jre openjdk-8-jdk
fi
BUILD=1 ./ci/ci.sh init
EOF

# Stage 2: full source tree (changes every commit, but installs are cached).
COPY . .

ENV CC=clang
ENV CXX=clang++-12
