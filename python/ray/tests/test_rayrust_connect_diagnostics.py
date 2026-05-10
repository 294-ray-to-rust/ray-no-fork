import os
import subprocess
import sys

import pytest

import ray
import ray._private.services as services


def _dump(msg):
    print(f"RAYRUST_CONNECT_DIAG {msg}", file=sys.stderr, flush=True)


def _node_id_hex(node_info):
    node_id = getattr(node_info, "node_id", None)
    hex_fn = getattr(node_id, "hex", None)
    if callable(hex_fn):
        return hex_fn()
    if isinstance(node_id, (bytes, bytearray)):
        return node_id.hex()
    return repr(node_id)


def _dump_node(prefix, node_info):
    _dump(
        "%s node_id=%s addr=%s port=%s raylet=%s object_store=%s "
        "temp_dir=%s session_dir=%s is_head=%s state=%s name=%s"
        % (
            prefix,
            _node_id_hex(node_info),
            getattr(node_info, "node_manager_address", None),
            getattr(node_info, "node_manager_port", None),
            getattr(node_info, "raylet_socket_name", None),
            getattr(node_info, "object_store_socket_name", None),
            getattr(node_info, "temp_dir", None),
            getattr(node_info, "session_dir", None),
            getattr(node_info, "is_head_node", None),
            getattr(node_info, "state", None),
            getattr(node_info, "node_name", None),
        )
    )


def _dump_raylet_processes():
    try:
        out = subprocess.check_output(
            ["bash", "-lc", "pgrep -af 'raylet|ray::' || true"],
            text=True,
            stderr=subprocess.STDOUT,
            timeout=10,
        )
    except Exception as exc:  # pragma: no cover - diagnostic only
        _dump(f"raylet_process_dump_error={exc!r}")
        return
    for line in out.splitlines():
        _dump(f"process {line}")
        pid = line.split(maxsplit=1)[0]
        if not pid.isdigit():
            continue
        try:
            raw = open(f"/proc/{pid}/cmdline", "rb").read()
            cmdline = raw.replace(b"\0", b" ").decode("utf-8", "replace")
            _dump(f"cmdline[{pid}]={cmdline}")
        except Exception as exc:  # pragma: no cover - diagnostic only
            _dump(f"cmdline_error[{pid}]={exc!r}")


def _dump_ray_sessions():
    try:
        out = subprocess.check_output(
            [
                "bash",
                "-lc",
                'ls -ld /tmp/ray/session* 2>/dev/null || true; '
                'for d in /tmp/ray/session_*; do '
                '[ -d "$d" ] || continue; '
                'echo SESSION "$d"; '
                'ls -l "$d/sockets" 2>/dev/null || true; '
                'done',
            ],
            text=True,
            stderr=subprocess.STDOUT,
            timeout=10,
        )
    except Exception as exc:  # pragma: no cover - diagnostic only
        _dump(f"ray_session_dump_error={exc!r}")
        return
    for line in out.splitlines():
        _dump(f"session {line}")


@pytest.mark.parametrize(
    "ray_start_cluster",
    [{"include_dashboard": True}],
    indirect=True,
)
def test_rayrust_connect_only_selection_diagnostics(ray_start_cluster):
    """Focused RayRust diagnostic for placement-group connect-only stalls.

    This intentionally stops after connect-only driver initialization. The
    goal is to make the selected node, GCS rows, local raylet processes, and
    Rust GcsClient boundary visible in Buildkite before more selector fixes.
    """

    os.environ["RAYRUST_CONNECT_DEBUG"] = "1"
    cluster = ray_start_cluster
    for _ in range(2):
        cluster.add_node(num_cpus=4)

    _dump(f"cluster_address={cluster.address}")
    _dump_raylet_processes()
    _dump_ray_sessions()

    gcs_client = ray._raylet.GcsClient(address=cluster.address)
    try:
        all_nodes = list(gcs_client.get_all_node_info().values())
    except Exception as exc:
        _dump(f"all_node_info_error={type(exc).__name__} {exc!r}")
        raise
    _dump(f"all_node_info_count={len(all_nodes)}")
    for idx, node_info in enumerate(all_nodes):
        _dump_node(f"all[{idx}]", node_info)

    try:
        selected = services.get_node_to_connect_for_driver(
            gcs_client, timeout_seconds=10
        )
    except Exception as exc:
        _dump(f"selected_error={type(exc).__name__} {exc!r}")
        raise
    _dump_node("selected", selected)

    try:
        ray.init(address=cluster.address)
    except Exception as exc:
        _dump(f"ray_init_error={type(exc).__name__} {exc!r}")
        raise
    _dump("ray_init_ok")


if __name__ == "__main__":
    sys.exit(pytest.main(["-sv", __file__]))
