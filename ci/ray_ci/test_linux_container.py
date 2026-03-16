import os
import sys

import pytest

from ci.ray_ci.linux_container import LinuxContainer


def test_get_docker_image() -> None:
    assert (
        LinuxContainer("test")._get_docker_image()
        == "029272617770.dkr.ecr.us-west-2.amazonaws.com/rayproject/citemp:test"
    )


def test_get_run_command() -> None:
    command = " ".join(LinuxContainer("test").get_run_command(["hi", "hello"]))
    assert "-env BUILDKITE_JOB_ID" in command
    assert "--cap-add SYS_PTRACE" in command
    assert "/bin/bash -iecuo pipefail -- hi\nhello" in command


def test_get_run_command_tmpfs() -> None:
    container = LinuxContainer("test", tmp_filesystem="tmpfs")
    command = " ".join(container.get_run_command(["hi", "hello"]))
    assert "--mount type=tmpfs,destination=/tmp" in command


def test_get_artifact_mount_default() -> None:
    """When RAYCI_ARTIFACT_DIR is unset, falls back to /tmp/artifacts."""
    old = os.environ.pop("RAYCI_ARTIFACT_DIR", None)
    try:
        host, container = LinuxContainer("test").get_artifact_mount()
        assert host == "/tmp/artifacts"
        assert container == "/artifact-mount"
    finally:
        if old is not None:
            os.environ["RAYCI_ARTIFACT_DIR"] = old


def test_get_artifact_mount_custom() -> None:
    """When RAYCI_ARTIFACT_DIR is set, uses the custom path."""
    old = os.environ.get("RAYCI_ARTIFACT_DIR")
    os.environ["RAYCI_ARTIFACT_DIR"] = "/scratch/artifacts/agent-1"
    try:
        host, container = LinuxContainer("test").get_artifact_mount()
        assert host == "/scratch/artifacts/agent-1"
        assert container == "/artifact-mount"
    finally:
        if old is not None:
            os.environ["RAYCI_ARTIFACT_DIR"] = old
        else:
            del os.environ["RAYCI_ARTIFACT_DIR"]


if __name__ == "__main__":
    sys.exit(pytest.main(["-v", __file__]))
