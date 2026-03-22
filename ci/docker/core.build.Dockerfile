ARG DOCKER_IMAGE_BASE_BUILD=cr.ray.io/rayproject/oss-ci-base_build-py3.10
FROM $DOCKER_IMAGE_BASE_BUILD

ARG PIP_INDEX_URL=""
ARG PIP_TRUSTED_HOST=""

ARG RAYCI_IS_GPU_BUILD=false

SHELL ["/bin/bash", "-ice"]

# Copy requirements files first — these rarely change, so pip install
# caches across commits.
COPY python/requirements.txt python/requirements.txt
COPY python/requirements_compiled.txt python/requirements_compiled.txt
COPY python/requirements/test-requirements.txt python/requirements/test-requirements.txt
COPY python/requirements/ml/dl-cpu-requirements.txt python/requirements/ml/dl-cpu-requirements.txt
COPY python/requirements/ml/dl-gpu-requirements.txt python/requirements/ml/dl-gpu-requirements.txt

RUN <<EOF
#!/bin/bash
set -euo pipefail
pip install -U \
    -c python/requirements_compiled.txt \
    -r python/requirements.txt \
    -r python/requirements/test-requirements.txt \
    -r python/requirements/ml/dl-cpu-requirements.txt
if [[ "$RAYCI_IS_GPU_BUILD" == "true" ]]; then
  pip install -Ur python/requirements/ml/dl-gpu-requirements.txt
fi
EOF

# Now copy everything else (install script, etc.)
COPY . .

# Incremental pass: install-dependencies.sh will skip already-satisfied
# pip packages and handle any remaining non-pip setup.
RUN DL=1 ./ci/env/install-dependencies.sh
