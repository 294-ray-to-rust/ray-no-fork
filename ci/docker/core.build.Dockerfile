ARG DOCKER_IMAGE_BASE_BUILD=cr.ray.io/rayproject/oss-ci-base_build-py3.10
FROM $DOCKER_IMAGE_BASE_BUILD

ARG PIP_INDEX_URL=""
ARG PIP_TRUSTED_HOST=""

ARG RAYCI_IS_GPU_BUILD=false

SHELL ["/bin/bash", "-ice"]

# --- Dependency files (wanda srcs) ---
COPY ci/ ci/
COPY python/requirements.txt python/requirements.txt
COPY python/requirements_compiled.txt python/requirements_compiled.txt
COPY python/requirements/test-requirements.txt python/requirements/test-requirements.txt
COPY python/requirements/ml/dl-cpu-requirements.txt python/requirements/ml/dl-cpu-requirements.txt
COPY python/requirements/ml/dl-gpu-requirements.txt python/requirements/ml/dl-gpu-requirements.txt

# DL CPU deps (cached unless dl-cpu-requirements.txt changes).
RUN pip install -U -c python/requirements_compiled.txt \
    -r python/requirements/ml/dl-cpu-requirements.txt

# DL GPU deps (only when building GPU variant).
RUN <<EOF
#!/bin/bash
set -euo pipefail
if [[ "$RAYCI_IS_GPU_BUILD" == "true" ]]; then
  pip install -Ur ./python/requirements/ml/dl-gpu-requirements.txt
fi
EOF

# Remaining install-dependencies.sh setup (mostly no-ops since
# base_build already has everything; DL packages already installed).
RUN bash -ic 'DL=1 ./ci/env/install-dependencies.sh'

# Full source tree.
COPY . .
