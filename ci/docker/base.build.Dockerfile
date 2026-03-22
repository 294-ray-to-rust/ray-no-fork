ARG DOCKER_IMAGE_BASE_TEST=cr.ray.io/rayproject/oss-ci-base_test
FROM $DOCKER_IMAGE_BASE_TEST

ARG APT_PROXY=""
RUN if [ -n "$APT_PROXY" ]; then \
      echo "Acquire::http::Proxy \"$APT_PROXY\";" > /etc/apt/apt.conf.d/01proxy; \
    fi

ARG PIP_INDEX_URL=""
ARG PIP_TRUSTED_HOST=""

ARG RAYCI_DISABLE_JAVA=false

# Install apt packages before COPY so this layer caches across code changes.
RUN <<EOF
#!/bin/bash -i
set -euo pipefail
if [[ "$RAYCI_DISABLE_JAVA" != "true" ]]; then
    apt-get update -y
    apt-get install -y -qq maven openjdk-8-jre openjdk-8-jdk
fi
EOF

# Copy requirements files first — these rarely change, so pip install
# caches across commits.
COPY python/requirements.txt python/requirements.txt
COPY python/requirements_compiled.txt python/requirements_compiled.txt
COPY python/requirements/test-requirements.txt python/requirements/test-requirements.txt

RUN pip install -U \
    -c python/requirements_compiled.txt \
    -r python/requirements.txt \
    -r python/requirements/test-requirements.txt

# Now copy everything else (scripts, .bazelrc, etc.) — changes every commit
# but the expensive pip install above is already cached.
COPY . .

# Incremental pass: installs any remaining deps (nvm, node, etc.) and
# verifies pip packages. pip will skip already-satisfied requirements.
RUN BUILD=1 ./ci/ci.sh init

ENV CC=clang
ENV CXX=clang++-12
