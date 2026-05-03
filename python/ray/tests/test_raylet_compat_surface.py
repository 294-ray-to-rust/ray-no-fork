"""Compatibility surface checks for the Rust ``ray._raylet`` module.

The Rust extension replaces the legacy Cython ``python/ray/_raylet.pyx``
module.  Most Python callers do not import the extension through a typed API;
they discover module constants, classes, methods, and descriptor attributes at
runtime.  This test is intentionally cheap and fail-fast: it checks that the
compatibility surface exists and has the rough shape expected by legacy Python
code before heavier smoke tests spend minutes finding the same issue indirectly.
"""

import inspect
import sys

import pytest

import ray._raylet as raylet

MODULE_CALLABLES = {
    "_get_actor_serialized_owner_address_or_none",
    "build_address",
    "compute_task_id",
    "del_key_prefix_from_storage",
    "get_authentication_mode",
    "get_port_filename",
    "get_ray_commit",
    "get_ray_version",
    "get_session_key_from_storage",
    "getproctitle",
    "is_initialized",
    "is_ipv6",
    "mark_initialized",
    "mark_shutdown",
    "maybe_initialize_job_config",
    "node_ip_address_from_perspective",
    "node_labels_match_selector",
    "parse_address",
    "persist_port",
    "raise_if_dependency_failed",
    "raise_sys_exit_with_custom_error_message",
    "serialize_retry_exception_allowlist",
    "setproctitle",
    "split_buffer",
    "start_cluster",
    "unpack_pickle5_buffers",
    "validate_authentication_token",
    "wait_for_persisted_port",
}

MODULE_CLASSES = {
    "ActorClassID",
    "ActorID",
    "AuthenticationMode",
    "Buffer",
    "ClusterID",
    "Config",
    "CoreWorker",
    "DynamicObjectRefGenerator",
    "FunctionDescriptor",
    "FunctionID",
    "GcsClient",
    "GcsClientOptions",
    "GcsErrorSubscriber",
    "GcsLogSubscriber",
    "GlobalStateAccessor",
    "JobID",
    "Language",
    "MessagePackSerializedObject",
    "NodeID",
    "NumReturnsWarning",
    "ObjectID",
    "ObjectRef",
    "ObjectRefGenerator",
    "ObjectRefStreamEndOfStreamError",
    "Pickle5SerializedObject",
    "PlacementGroupID",
    "PythonFunctionDescriptor",
    "RawSerializedObject",
    "SerializedObject",
    "SerializedRayObject",
    "StreamRedirector",
    "StreamingGeneratorStats",
    "TaskID",
    "UniqueID",
    "WorkerID",
    "WorkerType",
}

MODULE_CONSTANTS = {
    "GCS_AUTOSCALER_STATE_NAMESPACE",
    "GCS_AUTOSCALER_V2_ENABLED_KEY",
    "GCS_PID_KEY",
    "GCS_SERVER_PORT_NAME",
    "GRPC_STATUS_CODE_DEADLINE_EXCEEDED",
    "GRPC_STATUS_CODE_RESOURCE_EXHAUSTED",
    "GRPC_STATUS_CODE_UNAVAILABLE",
    "GRPC_STATUS_CODE_UNIMPLEMENTED",
    "GRPC_STATUS_CODE_UNKNOWN",
    "METRICS_AGENT_PORT_NAME",
    "METRICS_EXPORT_PORT_NAME",
    "NODE_MARKET_TYPE_ENV",
    "NODE_REGION_ENV",
    "NODE_TYPE_NAME_ENV",
    "NODE_ZONE_ENV",
    "OPTIMIZED",
    "RAY_INTERNAL_NAMESPACE_PREFIX",
    "RAY_NODE_ACCELERATOR_TYPE_KEY",
    "RAY_NODE_GROUP_KEY",
    "RAY_NODE_MARKET_TYPE_KEY",
    "RAY_NODE_REGION_KEY",
    "RAY_NODE_TPU_POD_TYPE_KEY",
    "RAY_NODE_TPU_SLICE_NAME_KEY",
    "RAY_NODE_TPU_TOPOLOGY_KEY",
    "RAY_NODE_TPU_WORKER_ID_KEY",
    "RAY_NODE_ZONE_KEY",
    "RAY_VERSION",
    "RESOURCE_UNIT_SCALING",
    "RUNTIME_ENV_AGENT_PORT_NAME",
    "STREAMING_GENERATOR_RETURN",
    "WORKER_PROCESS_SETUP_HOOK_KEY_NAME_GCS",
}

CLASS_MEMBERS = {
    "CoreWorker": {
        "create_placement_group",
        "get_current_job_id",
        "get_current_node_id",
        "get_job_config",
        "get_named_actor_handle",
        "get_worker_id",
        "kill_actor",
        "shutdown_driver",
        "submit_task",
        "wait",
    },
    "GcsClient": {
        "address",
        "async_internal_kv_del",
        "async_internal_kv_exists",
        "async_internal_kv_get",
        "async_internal_kv_keys",
        "async_internal_kv_put",
        "check_alive",
        "cluster_id",
        "drain_nodes",
        "get_all_node_info",
        "get_job_info",
        "internal_kv_del",
        "internal_kv_exists",
        "internal_kv_get",
        "internal_kv_keys",
        "internal_kv_put",
    },
    "GlobalStateAccessor": {
        "connect",
        "get_next_job_id",
        "get_node",
        "get_node_table",
        "get_system_config",
        "internal_kv_get",
        "internal_kv_put",
    },
    "Language": {"value"},
    "StreamRedirector": {"redirect_stderr", "redirect_stdout"},
}

ID_CLASSES = {
    "ActorClassID",
    "ActorID",
    "ClusterID",
    "FunctionID",
    "JobID",
    "NodeID",
    "ObjectID",
    "PlacementGroupID",
    "TaskID",
    "UniqueID",
    "WorkerID",
}

DESCRIPTOR_CLASSES = {
    "FunctionDescriptor",
    "PythonFunctionDescriptor",
}


def _missing(names, obj):
    present = set(dir(obj))
    return sorted(name for name in names if name not in present)


def test_module_public_symbols_are_present():
    expected = MODULE_CALLABLES | MODULE_CLASSES | MODULE_CONSTANTS
    assert not _missing(expected, raylet)


@pytest.mark.parametrize("name", sorted(MODULE_CALLABLES))
def test_module_callable_shape(name):
    assert callable(getattr(raylet, name)), name


@pytest.mark.parametrize("name", sorted(MODULE_CLASSES))
def test_module_class_shape(name):
    value = getattr(raylet, name)
    assert inspect.isclass(value), name


@pytest.mark.parametrize("name", sorted(MODULE_CONSTANTS))
def test_module_constant_shape(name):
    assert not callable(getattr(raylet, name)), name


@pytest.mark.parametrize("class_name, members", sorted(CLASS_MEMBERS.items()))
def test_class_members_are_present(class_name, members):
    cls = getattr(raylet, class_name)
    assert not _missing(members, cls), class_name


@pytest.mark.parametrize("class_name", sorted(ID_CLASSES))
def test_id_class_shape_and_nil_binary(class_name):
    cls = getattr(raylet, class_name)
    assert callable(cls.nil)
    assert callable(cls.from_hex)
    assert callable(cls.from_random)
    assert callable(cls.size)

    value = cls.nil()
    assert hasattr(value, "binary")
    assert hasattr(value, "hex")
    assert hasattr(value, "is_nil")
    assert isinstance(value.binary(), bytes)
    assert isinstance(value.hex(), str)
    assert value.is_nil() is True


@pytest.mark.parametrize("class_name", sorted(DESCRIPTOR_CLASSES))
def test_function_descriptor_shape_and_function_id(class_name):
    cls = getattr(raylet, class_name)
    descriptor = cls.from_function("module", "function")

    assert hasattr(descriptor, "function_id")
    function_id = descriptor.function_id
    assert hasattr(function_id, "binary")
    assert isinstance(function_id.binary(), bytes)

    assert hasattr(descriptor, "split")
    split_value = descriptor.split()
    assert isinstance(split_value, tuple)
    assert len(split_value) >= 2


def test_generic_stub_dumps_shape():
    assert callable(raylet.GenericStub.dumps)
    assert isinstance(raylet.GenericStub.dumps({"ok": True}), bytes)


def test_dashboard_dependency_import_surface():
    import aiohttp  # noqa: F401


def test_gcs_subscriber_empty_poll_shape():
    error_subscriber = raylet.GcsErrorSubscriber("127.0.0.1:0")
    assert error_subscriber.poll(timeout=0.001) == (None, None)

    log_subscriber = raylet.GcsLogSubscriber("127.0.0.1:0")
    assert log_subscriber.poll(timeout=0.001) is None


if __name__ == "__main__":
    sys.exit(pytest.main(["-sv", __file__]))
