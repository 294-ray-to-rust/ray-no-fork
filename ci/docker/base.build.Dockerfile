ARG DOCKER_IMAGE_BASE_TEST=cr.ray.io/rayproject/oss-ci-base_test
FROM $DOCKER_IMAGE_BASE_TEST

ARG APT_PROXY=""
RUN if [ -n "$APT_PROXY" ]; then \
      echo "Acquire::http::Proxy \"$APT_PROXY\";" > /etc/apt/apt.conf.d/01proxy; \
    fi

ARG PIP_INDEX_URL=""
ARG PIP_TRUSTED_HOST=""

# Make pip available without interactive bash (miniforge installed in base_test).
ENV PATH="/opt/miniforge/bin:${PATH}"

# Java layer — only busts when RAYCI_DISABLE_JAVA arg changes.
ARG RAYCI_DISABLE_JAVA=false
RUN <<EOF
#!/bin/bash
set -euo pipefail
if [[ "$RAYCI_DISABLE_JAVA" != "true" ]]; then
    apt-get update -y
    apt-get install -y -qq maven openjdk-8-jre openjdk-8-jdk
fi
EOF

# --- Dependency files (wanda srcs) ---
COPY ci/ ci/
COPY .bazelrc .bazelrc
COPY python/requirements.txt python/requirements.txt
COPY python/requirements_compiled.txt python/requirements_compiled.txt
COPY python/requirements/test-requirements.txt python/requirements/test-requirements.txt

# Base deps (~2.4 min, cached unless requirements.txt changes).
RUN pip install -U -c python/requirements_compiled.txt \
    -r python/requirements.txt

# Test deps (~9 min, cached unless test-requirements.txt changes).
RUN pip install -U -c python/requirements_compiled.txt \
    -r python/requirements/test-requirements.txt

# Remaining setup: LLVM, node, bazel config, thirdparty_files.
# pip packages are already installed so install_pip_packages is a no-op.
RUN bash -ic 'BUILD=1 ./ci/ci.sh init'

# Full source tree.
COPY . .

ENV CC=clang
ENV CXX=clang++-12
