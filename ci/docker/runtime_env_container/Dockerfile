ARG BASE_IMAGE
FROM $BASE_IMAGE

ARG APT_PROXY=""
RUN if [ -n "$APT_PROXY" ]; then \
      echo "Acquire::http::Proxy \"$APT_PROXY\";" > /etc/apt/apt.conf.d/01proxy; \
    fi

COPY python/ray/tests/runtime_env_container/ /home/ray/tests/

# Install podman
RUN pip install --no-cache-dir -c /home/ray/requirements_compiled.txt httpx
RUN sudo apt-get update && sudo apt-get install podman -y
