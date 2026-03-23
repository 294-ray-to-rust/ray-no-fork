# syntax=docker/dockerfile:1.3-labs

ARG DOCKER_IMAGE_BASE_TEST=cr.ray.io/rayproject/oss-ci-base_test
FROM $DOCKER_IMAGE_BASE_TEST

ARG PIP_INDEX_URL=""
ARG PIP_TRUSTED_HOST=""

ENV PATH="/opt/miniforge/bin:${PATH}"

# --- Dependency files (wanda srcs) ---
COPY ci/ ci/
COPY .bazelrc .bazelrc
COPY python/requirements.txt python/requirements.txt
COPY python/requirements_compiled.txt python/requirements_compiled.txt
COPY python/requirements/test-requirements.txt python/requirements/test-requirements.txt
COPY python/requirements/ml/ python/requirements/ml/

# Base deps.
RUN pip install -U -c python/requirements_compiled.txt \
    -r python/requirements.txt

# Test deps.
RUN pip install -U -c python/requirements_compiled.txt \
    -r python/requirements/test-requirements.txt

# ML deps (rllib, train, tune, dl-cpu, core).
RUN pip install -U -c python/requirements_compiled.txt \
    -r python/requirements/ml/rllib-requirements.txt \
    -r python/requirements/ml/rllib-test-requirements.txt \
    -r python/requirements/ml/train-requirements.txt \
    -r python/requirements/ml/train-test-requirements.txt \
    -r python/requirements/ml/tune-requirements.txt \
    -r python/requirements/ml/tune-test-requirements.txt \
    -r python/requirements/ml/dl-cpu-requirements.txt \
    -r python/requirements/ml/core-requirements.txt

# Remaining setup: LLVM, node, bazel config, thirdparty_files.
RUN bash -ic 'BUILD=1 ./ci/ci.sh init'

# Second pass with ML flags for any ML-specific non-pip setup.
RUN bash --login -ic 'RLLIB_TESTING=1 TRAIN_TESTING=1 TUNE_TESTING=1 ./ci/env/install-dependencies.sh'

RUN pip uninstall -y ray

# Full source tree.
COPY . .
