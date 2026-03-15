import base64
import io
import json
import logging
import os
import subprocess
import sys
import tempfile
from math import ceil
from pathlib import Path
from typing import List

import ci.ray_ci.bazel_sharding as bazel_sharding

from ray_release.bazel import bazel_runfile
from ray_release.configs.global_config import init_global_config
from ray_release.test import Test, TestState

GLOBAL_CONFIG_FILE = (
    os.environ.get("RAYCI_GLOBAL_CONFIG") or "ci/ray_ci/oss_config.yaml"
)
RAY_VERSION = "3.0.0.dev0"


def ci_init() -> None:
    """
    Initialize global config
    """
    init_global_config(bazel_runfile(GLOBAL_CONFIG_FILE))


def chunk_into_n(list: List[str], n: int) -> List[List[str]]:
    """
    Chunk a list into n chunks
    """
    size = ceil(len(list) / n)
    return [list[x * size : x * size + size] for x in range(n)]


def shard_tests(
    test_targets: List[str],
    shard_count: int,
    shard_id: int,
) -> List[str]:
    """
    Shard tests into N shards and return the shard corresponding to shard_id
    """
    return bazel_sharding.main(test_targets, index=shard_id, count=shard_count)


def docker_login(registry: str) -> None:
    """Login to a container registry, auto-detecting the auth mechanism.

    Supports:
    - ECR (*.dkr.ecr.*.amazonaws.com): uses boto3 for auth
    - GHCR (ghcr.io): uses GITHUB_TOKEN or GHCR_TOKEN env var
    - Other: logs a warning and skips login
    """
    if ".dkr.ecr." in registry and ".amazonaws.com" in registry:
        _ecr_docker_login(registry)
    elif "ghcr.io" in registry:
        _ghcr_docker_login(registry)
    else:
        logger.warning("Unknown registry type: %s, skipping docker login", registry)


def _ecr_docker_login(docker_ecr: str) -> None:
    """Login to ECR with AWS credentials."""
    import boto3

    token = boto3.client("ecr", region_name="us-west-2").get_authorization_token()
    user, password = (
        base64.b64decode(token["authorizationData"][0]["authorizationToken"])
        .decode("utf-8")
        .split(":")
    )
    _docker_login_with_token(docker_ecr, user, password)


def _ghcr_docker_login(registry: str) -> None:
    """Login to GHCR using GITHUB_TOKEN or GHCR_TOKEN env var.

    Falls back to existing Docker credentials if no token env var is set.
    """
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GHCR_TOKEN", "")
    if not token:
        if _docker_config_has_auth(registry):
            logger.info(
                "No GITHUB_TOKEN/GHCR_TOKEN env var, but Docker is already "
                "authenticated to %s — skipping login",
                registry,
            )
            return
        logger.warning(
            "No GITHUB_TOKEN/GHCR_TOKEN and no Docker config for %s. "
            "Proceeding without auth (images must be locally cached).",
            registry,
        )
        return
    _docker_login_with_token(registry, "USERNAME", token)


def _docker_config_has_auth(registry: str) -> bool:
    """Check if Docker config already has credentials for the given registry."""
    config_path = Path.home() / ".docker" / "config.json"
    if not config_path.exists():
        return False
    try:
        config = json.loads(config_path.read_text())
        auths = config.get("auths", {})
        return registry in auths or f"https://{registry}" in auths
    except (json.JSONDecodeError, OSError):
        return False


def _docker_login_with_token(registry: str, user: str, password: str) -> None:
    """Run docker login with the given credentials via stdin."""
    with tempfile.TemporaryFile() as f:
        f.write(bytes(password, "utf-8"))
        f.flush()
        f.seek(0)

        subprocess.run(
            [
                "docker",
                "login",
                "--username",
                user,
                "--password-stdin",
                registry,
            ],
            stdin=f,
            stdout=sys.stdout,
            stderr=sys.stderr,
            check=True,
        )


# Keep backward-compatible alias
ecr_docker_login = _ecr_docker_login


def docker_pull(image: str) -> None:
    """
    Pull docker image
    """
    subprocess.run(
        ["docker", "pull", image],
        stdout=sys.stdout,
        stderr=sys.stderr,
        check=True,
    )


def get_flaky_test_names(prefix: str) -> List[str]:
    """
    Query all flaky tests with specified prefix.

    Args:
        prefix: A prefix to filter by.

    Returns:
        List[str]: List of test names.
    """
    tests = Test.gen_from_s3(prefix)
    # Filter tests by test state
    state = TestState.FLAKY
    test_names = [t.get_name() for t in tests if t.get_state() == state]

    # Remove prefixes.
    for i in range(len(test_names)):
        test = test_names[i]
        if test.startswith(prefix):
            test_names[i] = test[len(prefix) :]

    return test_names


def filter_tests(
    input: io.TextIOBase, output: io.TextIOBase, prefix: str, state_filter: str
):
    """
    Filter flaky tests from list of test targets.

    Args:
        input: Input stream, each test name in one line.
        output: Output stream, each test name in one line.
        prefix: Prefix to query tests with.
        state_filter: Options to filter tests: "flaky" or "-flaky" tests.
    """
    # Valid prefix check
    if prefix not in ["darwin:", "linux:", "windows:"]:
        raise ValueError("Prefix must be one of 'darwin:', 'linux:', or 'windows:'.")

    # Valid filter choices check
    if state_filter not in ["flaky", "-flaky"]:
        raise ValueError("Filter option must be one of 'flaky' or '-flaky'.")

    # Obtain all existing tests with specified test state
    flaky_tests = set(get_flaky_test_names(prefix))

    # Filter these test from list of test targets based on user condition.
    for t in input:
        t = t.strip()
        if not t:
            continue

        hit = t in flaky_tests
        if state_filter == "-flaky":
            hit = not hit

        if hit:
            output.write(f"{t}\n")


logger = logging.getLogger()
logger.setLevel(logging.INFO)


def add_handlers(logger: logging.Logger):
    """
    Add handlers to logger
    """
    handler = logging.StreamHandler(stream=sys.stderr)
    formatter = logging.Formatter(
        fmt="[%(levelname)s %(asctime)s] %(filename)s: %(lineno)d  %(message)s"
    )
    handler.setFormatter(formatter)
    logger.addHandler(handler)


if not logger.hasHandlers():
    add_handlers(logger)
