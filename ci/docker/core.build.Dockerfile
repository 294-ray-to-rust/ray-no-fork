ARG DOCKER_IMAGE_BASE_BUILD=cr.ray.io/rayproject/oss-ci-base_build-py3.10
FROM $DOCKER_IMAGE_BASE_BUILD

ARG PIP_INDEX_URL=""
ARG PIP_TRUSTED_HOST=""

ARG RAYCI_IS_GPU_BUILD=false

SHELL ["/bin/bash", "-ice"]

# Stage 1: dependency specs from wanda srcs (base_build already has pip).
# .bazelrc persists from the base_build image; not in core.build srcs.
COPY ci/ ci/
COPY python/requirements.txt python/requirements.txt
COPY python/requirements_compiled.txt python/requirements_compiled.txt
COPY python/requirements/test-requirements.txt python/requirements/test-requirements.txt
COPY python/requirements/ml/dl-cpu-requirements.txt python/requirements/ml/dl-cpu-requirements.txt
COPY python/requirements/ml/dl-gpu-requirements.txt python/requirements/ml/dl-gpu-requirements.txt

RUN <<EOF
#!/bin/bash

set -euo pipefail

DL=1 ./ci/env/install-dependencies.sh

if [[ "$RAYCI_IS_GPU_BUILD" == "true" ]]; then
  pip install -Ur ./python/requirements/ml/dl-gpu-requirements.txt
fi

EOF

# Stage 2: full source tree.
COPY . .
