# syntax=docker/dockerfile:1.3-labs

ARG DOCKER_IMAGE_BASE_TEST=cr.ray.io/rayproject/oss-ci-base_test
FROM $DOCKER_IMAGE_BASE_TEST

ARG PIP_INDEX_URL=""
ARG PIP_TRUSTED_HOST=""

# Stage 1: dependency specs from wanda srcs (rarely change).
COPY ci/ ci/
COPY .bazelrc .bazelrc
COPY python/requirements.txt python/requirements.txt
COPY python/requirements_compiled.txt python/requirements_compiled.txt
COPY python/requirements/test-requirements.txt python/requirements/test-requirements.txt
COPY python/requirements/ml/ python/requirements/ml/

RUN <<EOF
#!/bin/bash -i

set -e

BUILD=1 ./ci/ci.sh init
RLLIB_TESTING=1 TRAIN_TESTING=1 TUNE_TESTING=1 bash --login -i ./ci/env/install-dependencies.sh

pip uninstall -y ray

EOF

# Stage 2: full source tree.
COPY . .
