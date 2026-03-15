ARG DOCKER_IMAGE_BASE_TEST=cr.ray.io/rayproject/oss-ci-base_test
FROM $DOCKER_IMAGE_BASE_TEST

ARG APT_PROXY=""
RUN if [ -n "$APT_PROXY" ]; then \
      echo "Acquire::http::Proxy \"$APT_PROXY\";" > /etc/apt/apt.conf.d/01proxy; \
    fi

ARG RAYCI_DISABLE_JAVA=false

COPY . .

RUN <<EOF
#!/bin/bash -i

set -euo pipefail

if [[ "$RAYCI_DISABLE_JAVA" != "true" ]]; then
    apt-get update -y
    apt-get install -y -qq maven openjdk-8-jre openjdk-8-jdk
fi

BUILD=1 ./ci/ci.sh init

EOF

ENV CC=clang
ENV CXX=clang++-12
