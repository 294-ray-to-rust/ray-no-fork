import base64
import io
import json
import logging
import sys
from typing import List
from unittest import mock

import pytest

from ci.ray_ci.utils import (
    chunk_into_n,
    docker_login,
    _docker_config_has_auth,
    _ecr_docker_login,
    _ghcr_docker_login,
    filter_tests,
    get_flaky_test_names,
)

from ray_release.test import Test


def test_chunk_into_n() -> None:
    assert chunk_into_n([1, 2, 3, 4, 5], 2) == [[1, 2, 3], [4, 5]]
    assert chunk_into_n([1, 2], 3) == [[1], [2], []]
    assert chunk_into_n([1, 2], 1) == [[1, 2]]


def test_ecr_docker_login() -> None:
    def _mock_subprocess_run(
        cmd: List[str],
        stdin=None,
        stdout=None,
        stderr=None,
        check=True,
    ) -> None:
        assert stdin.read() == b"password"

    mock_boto3 = mock.MagicMock()
    mock_boto3.client.return_value.get_authorization_token.return_value = {
        "authorizationData": [
            {"authorizationToken": base64.b64encode(b"AWS:password")},
        ],
    }

    with mock.patch.dict("sys.modules", {"boto3": mock_boto3}), \
         mock.patch("subprocess.run", side_effect=_mock_subprocess_run):
        _ecr_docker_login("docker_ecr")


def test_ghcr_docker_login() -> None:
    def _mock_subprocess_run(
        cmd: List[str],
        stdin=None,
        stdout=None,
        stderr=None,
        check=True,
    ) -> None:
        assert stdin.read() == b"my-ghcr-token"
        assert cmd[-1] == "ghcr.io"
        assert cmd[3] == "USERNAME"

    with mock.patch.dict(
        "os.environ", {"GITHUB_TOKEN": "my-ghcr-token"}, clear=False
    ), mock.patch("subprocess.run", side_effect=_mock_subprocess_run):
        _ghcr_docker_login("ghcr.io")


def test_ghcr_docker_login_missing_token(caplog) -> None:
    """When no token and no Docker config, log warning and return gracefully."""
    with mock.patch.dict(
        "os.environ", {}, clear=True
    ), mock.patch(
        "ci.ray_ci.utils._docker_config_has_auth", return_value=False
    ), caplog.at_level(logging.WARNING):
        _ghcr_docker_login("ghcr.io")  # should not raise
    assert "Proceeding without auth" in caplog.text


def test_ghcr_docker_login_falls_back_to_docker_config(caplog) -> None:
    """When no token env var is set but Docker config has auth, skip login."""
    with mock.patch.dict(
        "os.environ", {}, clear=True
    ), mock.patch(
        "ci.ray_ci.utils._docker_config_has_auth", return_value=True
    ), caplog.at_level(logging.INFO):
        _ghcr_docker_login("ghcr.io")  # should not raise
    assert "already authenticated" in caplog.text


def test_docker_config_has_auth_exact_match(tmp_path) -> None:
    docker_dir = tmp_path / ".docker"
    docker_dir.mkdir()
    config = {"auths": {"ghcr.io": {"auth": "dummytoken"}}}
    (docker_dir / "config.json").write_text(json.dumps(config))

    with mock.patch("ci.ray_ci.utils.Path.home", return_value=tmp_path):
        assert _docker_config_has_auth("ghcr.io") is True
        assert _docker_config_has_auth("docker.io") is False


def test_docker_config_has_auth_https_prefix(tmp_path) -> None:
    docker_dir = tmp_path / ".docker"
    docker_dir.mkdir()
    config = {"auths": {"https://ghcr.io": {"auth": "dummytoken"}}}
    (docker_dir / "config.json").write_text(json.dumps(config))

    with mock.patch("ci.ray_ci.utils.Path.home", return_value=tmp_path):
        assert _docker_config_has_auth("ghcr.io") is True


def test_docker_config_has_auth_no_file(tmp_path) -> None:
    with mock.patch("ci.ray_ci.utils.Path.home", return_value=tmp_path):
        assert _docker_config_has_auth("ghcr.io") is False


def test_docker_config_has_auth_invalid_json(tmp_path) -> None:
    docker_dir = tmp_path / ".docker"
    docker_dir.mkdir()
    (docker_dir / "config.json").write_text("not valid json")

    with mock.patch("ci.ray_ci.utils.Path.home", return_value=tmp_path):
        assert _docker_config_has_auth("ghcr.io") is False


def test_docker_login_dispatches_ecr() -> None:
    with mock.patch("ci.ray_ci.utils._ecr_docker_login") as mock_ecr:
        docker_login("029272617770.dkr.ecr.us-west-2.amazonaws.com")
        mock_ecr.assert_called_once_with(
            "029272617770.dkr.ecr.us-west-2.amazonaws.com"
        )


def test_docker_login_dispatches_ghcr() -> None:
    with mock.patch("ci.ray_ci.utils._ghcr_docker_login") as mock_ghcr:
        docker_login("ghcr.io")
        mock_ghcr.assert_called_once_with("ghcr.io")


def test_docker_login_unknown_registry(caplog) -> None:
    with caplog.at_level(logging.WARNING):
        docker_login("registry.example.com")
    assert "Unknown registry type" in caplog.text


def _make_test(name: str, state: str, team: str) -> Test:
    return Test(
        {
            "name": name,
            "state": state,
            "team": team,
        }
    )


@mock.patch("ray_release.test.Test.gen_from_s3")
def test_get_flaky_test_names(mock_gen_from_s3):
    mock_gen_from_s3.side_effect = (
        [
            _make_test("darwin://test_1", "flaky", "core"),
            _make_test("darwin://test_2", "flaky", "ci"),
            _make_test("darwin://test_3", "passing", "core"),
        ],
        [
            _make_test("linux://test_1", "flaky", "core"),
            _make_test("linux://test_2", "passing", "ci"),
        ],
    )
    flaky_test_names = get_flaky_test_names(
        prefix="darwin:",
    )
    assert flaky_test_names == ["//test_1", "//test_2"]
    flaky_test_names = get_flaky_test_names(
        prefix="linux:",
    )
    assert flaky_test_names == ["//test_1"]


@pytest.mark.parametrize(
    "state_filter, expected_value",
    [
        (
            "-flaky",
            "//test_3\n//test_4\n",
        ),
        (
            "flaky",
            "//test_1\n//test_2\n",
        ),
    ],
)
@mock.patch("ray_release.test.Test.gen_from_s3")
def test_filter_tests(mock_gen_from_s3, state_filter, expected_value):
    # Setup test input/output
    mock_gen_from_s3.side_effect = (
        [
            _make_test("darwin://test_1", "flaky", "core"),
            _make_test("darwin://test_2", "flaky", "ci"),
            _make_test("darwin://test_3", "passing", "core"),
            _make_test("darwin://test_4", "passing", "ci"),
        ],
    )
    test_targets = ["//test_1", "//test_2", "//test_3", "//test_4"]
    output = io.StringIO()

    filter_tests(io.StringIO("\n".join(test_targets)), output, "darwin:", state_filter)
    assert output.getvalue() == expected_value


@pytest.mark.parametrize(
    "state_filter, prefix, error_message",
    [
        (
            "wrong-option",  # invalid filter option
            "darwin:",
            "Filter option must be one of",
        ),
        ("-flaky", "wrong-prefix", "Prefix must be one of"),  # invalid prefix
    ],
)
def test_filter_tests_fail(state_filter, prefix, error_message):
    test_targets = ["//test_1", "//test_2", "//test_3", "//test_4"]
    output = io.StringIO()
    with pytest.raises(ValueError, match=error_message):
        filter_tests(io.StringIO("\n".join(test_targets)), output, prefix, state_filter)
    return


if __name__ == "__main__":
    sys.exit(pytest.main(["-v", __file__]))
