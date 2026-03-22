# syntax=docker/dockerfile:1.3-labs

ARG DOCKER_IMAGE_BASE_TEST=cr.ray.io/rayproject/oss-ci-base_test
FROM $DOCKER_IMAGE_BASE_TEST

ARG PIP_INDEX_URL=""
ARG PIP_TRUSTED_HOST=""

# Copy requirements files first — these rarely change, so pip install
# caches across commits.
COPY python/requirements.txt python/requirements.txt
COPY python/requirements_compiled.txt python/requirements_compiled.txt
COPY python/requirements/test-requirements.txt python/requirements/test-requirements.txt
COPY python/requirements/ml/rllib-requirements.txt python/requirements/ml/rllib-requirements.txt
COPY python/requirements/ml/rllib-test-requirements.txt python/requirements/ml/rllib-test-requirements.txt
COPY python/requirements/ml/train-requirements.txt python/requirements/ml/train-requirements.txt
COPY python/requirements/ml/train-test-requirements.txt python/requirements/ml/train-test-requirements.txt
COPY python/requirements/ml/tune-requirements.txt python/requirements/ml/tune-requirements.txt
COPY python/requirements/ml/tune-test-requirements.txt python/requirements/ml/tune-test-requirements.txt
COPY python/requirements/ml/dl-cpu-requirements.txt python/requirements/ml/dl-cpu-requirements.txt
COPY python/requirements/ml/core-requirements.txt python/requirements/ml/core-requirements.txt

RUN <<EOF
#!/bin/bash -i
set -e
pip install -U \
    -c python/requirements_compiled.txt \
    -r python/requirements.txt \
    -r python/requirements/test-requirements.txt \
    -r python/requirements/ml/rllib-requirements.txt \
    -r python/requirements/ml/rllib-test-requirements.txt \
    -r python/requirements/ml/train-requirements.txt \
    -r python/requirements/ml/train-test-requirements.txt \
    -r python/requirements/ml/tune-requirements.txt \
    -r python/requirements/ml/tune-test-requirements.txt \
    -r python/requirements/ml/dl-cpu-requirements.txt \
    -r python/requirements/ml/core-requirements.txt
EOF

# Now copy everything else (scripts, .bazelrc, etc.)
COPY . .

# Incremental pass: ci.sh init handles non-pip setup, install-dependencies.sh
# verifies remaining deps. pip will skip already-satisfied packages.
RUN <<EOF
#!/bin/bash -i
set -e
BUILD=1 ./ci/ci.sh init
RLLIB_TESTING=1 TRAIN_TESTING=1 TUNE_TESTING=1 bash --login -i ./ci/env/install-dependencies.sh
pip uninstall -y ray
EOF
