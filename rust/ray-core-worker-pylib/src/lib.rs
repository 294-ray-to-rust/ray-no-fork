// Copyright 2024 The Ray Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0

//! PyO3 Python bindings for Ray (_raylet.so replacement).
//!
//! Replaces `python/ray/_raylet.pyx` (Cython).
//! Will be built with maturin to produce `_raylet.so`.

pub mod common;
pub mod cluster;
pub mod ids;
pub mod object_ref;
pub mod core_worker;
pub mod gcs_client;
pub mod serialization;

// Re-export primary types for Rust consumers.
pub use common::{PyLanguage, PyWorkerType};
pub use core_worker::PyCoreWorker;
pub use gcs_client::PyGcsClient;
pub use ids::*;
pub use object_ref::PyObjectRef;

// ─── PyO3 module (only when the "python" feature is enabled) ─────────

#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use pyo3::types::{PyBytes, PyList, PyType};

#[cfg(feature = "python")]
#[pyclass(module = "_raylet")]
struct Config;

#[cfg(feature = "python")]
#[pymethods]
impl Config {
    #[new]
    fn new() -> Self {
        Config
    }

    fn __getattr__(&self, py: Python<'_>, _name: &str) -> PyResult<Py<PyAny>> {
        py.eval_bound("lambda *a, **kw: -1", None, None)
            .map(|value| value.unbind())
    }
}

#[cfg(feature = "python")]
#[pyclass(module = "_raylet")]
struct ObjectRefGenerator;

#[cfg(feature = "python")]
#[pymethods]
impl ObjectRefGenerator {
    #[new]
    fn new() -> Self {
        ObjectRefGenerator
    }
}

#[cfg(feature = "python")]
#[pyclass(module = "_raylet")]
struct DynamicObjectRefGenerator;

#[cfg(feature = "python")]
#[pymethods]
impl DynamicObjectRefGenerator {
    #[new]
    fn new() -> Self {
        DynamicObjectRefGenerator
    }
}

#[cfg(feature = "python")]
#[pyclass(module = "_raylet")]
struct GcsClientOptions {
    gcs_address: String,
    cluster_id_hex: Option<String>,
}

#[cfg(feature = "python")]
#[pymethods]
impl GcsClientOptions {
    #[new]
    #[pyo3(signature = (gcs_address = "", cluster_id_hex = None))]
    fn new(gcs_address: &str, cluster_id_hex: Option<String>) -> Self {
        GcsClientOptions {
            gcs_address: gcs_address.to_owned(),
            cluster_id_hex,
        }
    }

    #[classmethod]
    #[pyo3(signature = (gcs_address, cluster_id_hex = None, _allow_cluster_id_nil = true, _fetch_cluster_id_if_nil = false))]
    fn create(
        _cls: &Bound<'_, PyType>,
        gcs_address: &str,
        cluster_id_hex: Option<String>,
        _allow_cluster_id_nil: bool,
        _fetch_cluster_id_if_nil: bool,
    ) -> Self {
        GcsClientOptions {
            gcs_address: gcs_address.to_owned(),
            cluster_id_hex,
        }
    }

    #[getter]
    fn gcs_address(&self) -> &str {
        &self.gcs_address
    }

    #[getter]
    fn cluster_id_hex(&self) -> Option<&str> {
        self.cluster_id_hex.as_deref()
    }
}

#[cfg(feature = "python")]
#[pyclass(module = "_raylet")]
struct GlobalStateAccessor;

#[cfg(feature = "python")]
#[pyclass(module = "_raylet", subclass)]
struct SerializedObject {
    metadata: Py<PyAny>,
    contained_object_refs: Py<PyAny>,
}

#[cfg(feature = "python")]
impl SerializedObject {
    fn new_with_refs(
        py: Python<'_>,
        metadata: Py<PyAny>,
        contained_object_refs: Option<Py<PyAny>>,
    ) -> Self {
        let contained_object_refs =
            contained_object_refs.unwrap_or_else(|| PyList::empty_bound(py).unbind().into());
        SerializedObject {
            metadata,
            contained_object_refs,
        }
    }
}

#[cfg(feature = "python")]
#[pymethods]
impl SerializedObject {
    #[new]
    #[pyo3(signature = (metadata, contained_object_refs = None))]
    fn new(py: Python<'_>, metadata: Py<PyAny>, contained_object_refs: Option<Py<PyAny>>) -> Self {
        SerializedObject::new_with_refs(py, metadata, contained_object_refs)
    }

    #[getter]
    fn metadata(&self, py: Python<'_>) -> Py<PyAny> {
        self.metadata.clone_ref(py)
    }

    #[getter]
    fn contained_object_refs(&self, py: Python<'_>) -> Py<PyAny> {
        self.contained_object_refs.clone_ref(py)
    }

    #[getter]
    fn total_bytes(&self) -> PyResult<usize> {
        Err(pyo3::exceptions::PyNotImplementedError::new_err(
            "SerializedObject.total_bytes not implemented",
        ))
    }
}

#[cfg(feature = "python")]
#[pyclass(module = "_raylet", extends = SerializedObject)]
struct Pickle5SerializedObject {
    inband: Vec<u8>,
}

#[cfg(feature = "python")]
#[pymethods]
impl Pickle5SerializedObject {
    #[new]
    fn new(
        py: Python<'_>,
        metadata: Py<PyAny>,
        inband: &Bound<'_, PyAny>,
        _writer: Py<PyAny>,
        contained_object_refs: Py<PyAny>,
    ) -> PyResult<(Self, SerializedObject)> {
        Ok((
            Pickle5SerializedObject {
                inband: inband.extract()?,
            },
            SerializedObject::new_with_refs(py, metadata, Some(contained_object_refs)),
        ))
    }

    #[getter]
    fn total_bytes(&self) -> usize {
        self.inband.len()
    }
}

#[cfg(feature = "python")]
#[pyclass(module = "_raylet", extends = SerializedObject)]
struct MessagePackSerializedObject {
    msgpack_data: Vec<u8>,
    nested_bytes: Option<Vec<u8>>,
}

#[cfg(feature = "python")]
#[pymethods]
impl MessagePackSerializedObject {
    #[new]
    #[pyo3(signature = (metadata, msgpack_data, contained_object_refs, nest_serialized_object = None))]
    fn new(
        py: Python<'_>,
        metadata: Py<PyAny>,
        msgpack_data: &Bound<'_, PyAny>,
        contained_object_refs: Py<PyAny>,
        nest_serialized_object: Option<Py<PyAny>>,
    ) -> PyResult<(Self, SerializedObject)> {
        let nested_bytes = match nest_serialized_object {
            Some(obj) => Some(obj.bind(py).call_method0("to_bytes")?.extract()?),
            None => None,
        };
        Ok((
            MessagePackSerializedObject {
                msgpack_data: msgpack_data.extract()?,
                nested_bytes,
            },
            SerializedObject::new_with_refs(py, metadata, Some(contained_object_refs)),
        ))
    }

    #[getter]
    fn total_bytes(&self) -> usize {
        ray_common::constants::MESSAGE_PACK_OFFSET
            + self.msgpack_data.len()
            + self.nested_bytes.as_ref().map(Vec::len).unwrap_or(0)
    }

    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        let mut out = vec![0_u8; self.total_bytes()];
        // This is enough for Python-side smoke tests and preserves the Cython
        // layout: msgpack payload starts at MESSAGE_PACK_OFFSET, followed by
        // any nested serialized object bytes.
        let start = ray_common::constants::MESSAGE_PACK_OFFSET;
        out[start..start + self.msgpack_data.len()].copy_from_slice(&self.msgpack_data);
        if let Some(nested) = &self.nested_bytes {
            let nested_start = start + self.msgpack_data.len();
            out[nested_start..nested_start + nested.len()].copy_from_slice(nested);
        }
        PyBytes::new_bound(py, &out)
    }
}

#[cfg(feature = "python")]
#[pyclass(module = "_raylet", extends = SerializedObject)]
struct RawSerializedObject {
    value: Vec<u8>,
}

#[cfg(feature = "python")]
#[pyclass(module = "_raylet")]
struct GenericStub;

#[cfg(feature = "python")]
#[pymethods]
impl GenericStub {
    #[new]
    #[pyo3(signature = (*_args, **_kwargs))]
    fn new(
        _args: &Bound<'_, pyo3::types::PyTuple>,
        _kwargs: Option<&Bound<'_, pyo3::types::PyDict>>,
    ) -> Self {
        GenericStub
    }

    #[classmethod]
    fn instance(_cls: &Bound<'_, PyType>) -> Self {
        GenericStub
    }

    #[classmethod]
    #[pyo3(signature = (*_args, **_kwargs))]
    fn from_class(
        _cls: &Bound<'_, PyType>,
        _args: &Bound<'_, pyo3::types::PyTuple>,
        _kwargs: Option<&Bound<'_, pyo3::types::PyDict>>,
    ) -> Self {
        GenericStub
    }

    #[classmethod]
    #[pyo3(signature = (*_args, **_kwargs))]
    fn from_function(
        _cls: &Bound<'_, PyType>,
        _args: &Bound<'_, pyo3::types::PyTuple>,
        _kwargs: Option<&Bound<'_, pyo3::types::PyDict>>,
    ) -> Self {
        GenericStub
    }

    fn reset_cache(&self) {}

    fn __getattr__(&self, py: Python<'_>, _name: &str) -> PyResult<Py<PyAny>> {
        py.eval_bound("lambda *a, **kw: None", None, None)
            .map(|value| value.unbind())
    }
}

#[cfg(feature = "python")]
#[pyclass(module = "_raylet")]
struct ObjectRefStreamEndOfStreamError {
    message: String,
}

#[cfg(feature = "python")]
#[pymethods]
impl ObjectRefStreamEndOfStreamError {
    #[new]
    #[pyo3(signature = (message = ""))]
    fn new(message: &str) -> Self {
        ObjectRefStreamEndOfStreamError {
            message: message.to_owned(),
        }
    }

    fn __str__(&self) -> &str {
        &self.message
    }
}

#[cfg(feature = "python")]
#[pymethods]
impl RawSerializedObject {
    #[new]
    fn new(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<(Self, SerializedObject)> {
        Ok((
            RawSerializedObject {
                value: value.extract()?,
            },
            SerializedObject::new_with_refs(
                py,
                PyBytes::new_bound(py, b"RAW").unbind().into(),
                None,
            ),
        ))
    }

    #[getter]
    fn total_bytes(&self) -> usize {
        self.value.len()
    }

    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new_bound(py, &self.value)
    }
}

#[cfg(feature = "python")]
#[pymethods]
impl GlobalStateAccessor {
    #[new]
    fn new(_gcs_options: Option<Py<PyAny>>) -> Self {
        GlobalStateAccessor
    }

    fn connect(&self) -> bool {
        true
    }

    fn __getattr__(&self, py: Python<'_>, _name: &str) -> PyResult<Py<PyAny>> {
        py.eval_bound("lambda *a, **kw: []", None, None)
            .map(|value| value.unbind())
    }
}

#[cfg(feature = "python")]
#[pyfunction]
fn build_address(host: &str, port: &Bound<'_, PyAny>) -> PyResult<String> {
    let port = port.str()?.to_str()?.to_owned();
    if host.contains(':') && !host.starts_with('[') {
        Ok(format!("[{host}]:{port}"))
    } else {
        Ok(format!("{host}:{port}"))
    }
}

#[cfg(feature = "python")]
#[pyfunction]
fn parse_address(address: &str) -> Option<(String, String)> {
    if let Some(rest) = address.strip_prefix('[') {
        let (host, port) = rest.split_once("]:")?;
        return Some((host.to_owned(), port.to_owned()));
    }

    let (host, port) = address.rsplit_once(':')?;
    if host.contains(':') {
        None
    } else {
        Some((host.to_owned(), port.to_owned()))
    }
}

#[cfg(feature = "python")]
#[pyfunction]
fn is_ipv6(host: &str) -> bool {
    host.parse::<std::net::IpAddr>()
        .map(|ip| ip.is_ipv6())
        .unwrap_or_else(|_| host.contains(':'))
}

#[cfg(feature = "python")]
#[pyfunction]
fn node_ip_address_from_perspective(address: Option<&str>) -> String {
    let target = address.unwrap_or("8.8.8.8:53");
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if socket.connect(target).is_ok() {
            if let Ok(addr) = socket.local_addr() {
                return addr.ip().to_string();
            }
        }
    }
    "127.0.0.1".to_owned()
}

#[cfg(feature = "python")]
#[pyfunction]
fn get_port_filename(node_id: &str, port_name: &str) -> String {
    format!("{port_name}_{node_id}")
}

#[cfg(feature = "python")]
#[pyfunction]
fn persist_port(dir: &str, node_id: &str, port_name: &str, port: i32) -> PyResult<()> {
    let path = std::path::Path::new(dir).join(get_port_filename(node_id, port_name));
    std::fs::write(&path, port.to_string()).map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!(
            "failed to persist port to {}: {e}",
            path.display()
        ))
    })
}

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(signature = (dir, node_id, port_name, timeout_ms = 30000, poll_interval_ms = 100))]
fn wait_for_persisted_port(
    dir: &str,
    node_id: &str,
    port_name: &str,
    timeout_ms: u64,
    poll_interval_ms: u64,
) -> PyResult<i32> {
    let path = std::path::Path::new(dir).join(get_port_filename(node_id, port_name));
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);

    loop {
        match std::fs::read_to_string(&path) {
            Ok(value) => {
                return value.trim().parse::<i32>().map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(format!(
                        "invalid port value in {}: {e}",
                        path.display()
                    ))
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if std::time::Instant::now() >= deadline {
                    return Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                        "timed out waiting for persisted port {}",
                        path.display()
                    )));
                }
                std::thread::sleep(std::time::Duration::from_millis(poll_interval_ms));
            }
            Err(e) => {
                return Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "failed to read {}: {e}",
                    path.display()
                )));
            }
        }
    }
}

#[cfg(feature = "python")]
#[pyfunction]
fn get_session_key_from_storage(
    _host: &str,
    _port: u16,
    _username: Py<PyAny>,
    _password: Py<PyAny>,
    _use_ssl: bool,
    _config: Py<PyAny>,
    _key: &str,
) -> Option<Vec<u8>> {
    None
}

#[cfg(feature = "python")]
#[pyfunction]
fn del_key_prefix_from_storage(
    _host: &str,
    _port: u16,
    _username: Py<PyAny>,
    _password: Py<PyAny>,
    _use_ssl: bool,
    _key_prefix: &str,
) {
}

#[cfg(feature = "python")]
#[pyfunction]
fn get_authentication_mode() -> i32 {
    0
}

#[cfg(feature = "python")]
#[pyfunction]
fn validate_authentication_token(_provided_metadata: &str) -> bool {
    true
}

#[cfg(feature = "python")]
#[pyfunction]
fn node_labels_match_selector(
    node_labels: std::collections::HashMap<String, String>,
    selector: std::collections::HashMap<String, String>,
) -> bool {
    selector
        .iter()
        .all(|(key, value)| node_labels.get(key) == Some(value))
}

#[cfg(feature = "python")]
#[pyfunction]
fn raise_sys_exit_with_custom_error_message(message: &str) -> PyResult<()> {
    Err(pyo3::exceptions::PySystemExit::new_err(message.to_owned()))
}

#[cfg(feature = "python")]
#[pyfunction]
fn split_buffer<'py>(
    py: Python<'py>,
    data: &Bound<'_, PyAny>,
) -> PyResult<(Bound<'py, PyBytes>, Bound<'py, PyBytes>)> {
    let bytes: Vec<u8> = data.extract()?;
    let offset = ray_common::constants::MESSAGE_PACK_OFFSET.min(bytes.len());
    Ok((
        PyBytes::new_bound(py, &bytes[offset..]),
        PyBytes::new_bound(py, &[]),
    ))
}

#[cfg(feature = "python")]
#[pyfunction]
fn unpack_pickle5_buffers<'py>(
    py: Python<'py>,
    data: &Bound<'_, PyAny>,
) -> PyResult<(Bound<'py, PyBytes>, Vec<Bound<'py, PyBytes>>)> {
    let bytes: Vec<u8> = data.extract()?;
    Ok((PyBytes::new_bound(py, &bytes), Vec::new()))
}

#[cfg(feature = "python")]
#[pyfunction]
fn get_ray_version() -> &'static str {
    ray_common::constants::RAY_VERSION
}

#[cfg(feature = "python")]
#[pyfunction]
fn get_ray_commit() -> &'static str {
    // Will be replaced by build script in production.
    "unknown"
}

/// Check if Ray has been initialized (stub — always false until ray_init called).
#[cfg(feature = "python")]
#[pyfunction]
fn is_initialized() -> bool {
    INITIALIZED.load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(feature = "python")]
static INITIALIZED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Mark Ray as initialized. Called internally after successful init.
#[cfg(feature = "python")]
#[pyfunction]
fn mark_initialized() {
    INITIALIZED.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Mark Ray as shut down.
#[cfg(feature = "python")]
#[pyfunction]
fn mark_shutdown() {
    INITIALIZED.store(false, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(feature = "python")]
#[pymodule]
fn _raylet(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // ─── Module-level functions ──────────────────────────────────
    m.add_function(wrap_pyfunction!(get_ray_version, m)?)?;
    m.add_function(wrap_pyfunction!(get_ray_commit, m)?)?;
    m.add_function(wrap_pyfunction!(is_initialized, m)?)?;
    m.add_function(wrap_pyfunction!(mark_initialized, m)?)?;
    m.add_function(wrap_pyfunction!(mark_shutdown, m)?)?;
    m.add_function(wrap_pyfunction!(build_address, m)?)?;
    m.add_function(wrap_pyfunction!(parse_address, m)?)?;
    m.add_function(wrap_pyfunction!(is_ipv6, m)?)?;
    m.add_function(wrap_pyfunction!(node_ip_address_from_perspective, m)?)?;
    m.add_function(wrap_pyfunction!(get_port_filename, m)?)?;
    m.add_function(wrap_pyfunction!(persist_port, m)?)?;
    m.add_function(wrap_pyfunction!(wait_for_persisted_port, m)?)?;
    m.add_function(wrap_pyfunction!(get_session_key_from_storage, m)?)?;
    m.add_function(wrap_pyfunction!(del_key_prefix_from_storage, m)?)?;
    m.add_function(wrap_pyfunction!(get_authentication_mode, m)?)?;
    m.add_function(wrap_pyfunction!(validate_authentication_token, m)?)?;
    m.add_function(wrap_pyfunction!(node_labels_match_selector, m)?)?;
    m.add_function(wrap_pyfunction!(
        raise_sys_exit_with_custom_error_message,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(split_buffer, m)?)?;
    m.add_function(wrap_pyfunction!(unpack_pickle5_buffers, m)?)?;

    // ─── ID types ────────────────────────────────────────────────
    m.add_class::<ids::PyObjectID>()?;
    m.add_class::<ids::PyTaskID>()?;
    m.add_class::<ids::PyActorID>()?;
    m.add_class::<ids::PyJobID>()?;
    m.add_class::<ids::PyWorkerID>()?;
    m.add_class::<ids::PyNodeID>()?;
    m.add_class::<ids::PyPlacementGroupID>()?;
    m.add_class::<ids::PyActorClassID>()?;
    m.add_class::<ids::PyFunctionID>()?;
    m.add_class::<ids::PyUniqueID>()?;
    m.add_class::<ids::PyClusterID>()?;

    // ─── Enums ───────────────────────────────────────────────────
    m.add_class::<common::PyLanguage>()?;
    m.add_class::<common::PyWorkerType>()?;

    // ─── Core types ──────────────────────────────────────────────
    m.add_class::<object_ref::PyObjectRef>()?;
    m.add_class::<core_worker::PyCoreWorker>()?;
    m.add_class::<gcs_client::PyGcsClient>()?;
    m.add_class::<cluster::PyClusterHandle>()?;
    m.add_class::<GcsClientOptions>()?;
    m.add_class::<Config>()?;
    m.add_class::<ObjectRefGenerator>()?;
    m.add_class::<DynamicObjectRefGenerator>()?;
    m.add_class::<GlobalStateAccessor>()?;
    m.add_class::<SerializedObject>()?;
    m.add_class::<Pickle5SerializedObject>()?;
    m.add_class::<MessagePackSerializedObject>()?;
    m.add_class::<RawSerializedObject>()?;
    m.add_class::<GenericStub>()?;
    m.add_class::<ObjectRefStreamEndOfStreamError>()?;

    // Compatibility stubs for Python modules that import the legacy Cython
    // _raylet surface during startup. These are filled in as Rust parity grows.
    for name in [
        "AuthenticationTokenLoader",
        "Count",
        "CppFunctionDescriptor",
        "Gauge",
        "Histogram",
        "JavaFunctionDescriptor",
        "MessagePackSerializer",
        "Pickle5Writer",
        "PythonFunctionDescriptor",
        "RayletClient",
        "SerializedRayObject",
        "StreamRedirector",
        "StreamingGeneratorStats",
        "Sum",
    ] {
        m.add(name, m.getattr("GenericStub")?)?;
    }
    m.add(
        "AuthenticationMode",
        m.py().eval_bound(
            "type('AuthenticationMode', (), {'DISABLED': 0, 'TOKEN': 1})",
            None,
            None,
        )?,
    )?;

    // ─── Cluster functions ───────────────────────────────────────
    m.add_function(wrap_pyfunction!(cluster::start_cluster, m)?)?;

    // ─── Constants ───────────────────────────────────────────────
    m.add("RAY_VERSION", ray_common::constants::RAY_VERSION)?;
    m.add("WORKER_PROCESS_SETUP_HOOK_KEY_NAME_GCS", "FunctionsToRun")?;
    m.add("RESOURCE_UNIT_SCALING", 10000)?;
    m.add("IMPLICIT_RESOURCE_PREFIX", "node:__internal_implicit_resource_")?;
    m.add("STREAMING_GENERATOR_RETURN", -2)?;
    m.add("GCS_AUTOSCALER_STATE_NAMESPACE", "__autoscaler")?;
    m.add("GCS_AUTOSCALER_V2_ENABLED_KEY", "__autoscaler_v2_enabled")?;
    m.add("GCS_AUTOSCALER_CLUSTER_CONFIG_KEY", "__autoscaler_cluster_config")?;
    m.add("GCS_PID_KEY", "gcs_pid")?;
    m.add("NODE_TYPE_NAME_ENV", "RAY_NODE_TYPE_NAME")?;
    m.add("NODE_MARKET_TYPE_ENV", "RAY_NODE_MARKET_TYPE")?;
    m.add("NODE_REGION_ENV", "RAY_NODE_REGION")?;
    m.add("NODE_ZONE_ENV", "RAY_NODE_ZONE")?;
    m.add("RAY_NODE_ACCELERATOR_TYPE_KEY", "ray.io/accelerator-type")?;
    m.add("RAY_NODE_MARKET_TYPE_KEY", "ray.io/market-type")?;
    m.add("RAY_NODE_REGION_KEY", "ray.io/availability-region")?;
    m.add("RAY_NODE_ZONE_KEY", "ray.io/availability-zone")?;
    m.add("RAY_NODE_GROUP_KEY", "ray.io/node-group")?;
    m.add("RAY_NODE_TPU_TOPOLOGY_KEY", "ray.io/tpu-topology")?;
    m.add("RAY_NODE_TPU_SLICE_NAME_KEY", "ray.io/tpu-slice-name")?;
    m.add("RAY_NODE_TPU_WORKER_ID_KEY", "ray.io/tpu-worker-id")?;
    m.add("RAY_NODE_TPU_POD_TYPE_KEY", "ray.io/tpu-pod-type")?;
    m.add("RAY_INTERNAL_NAMESPACE_PREFIX", "_ray_internal_")?;
    m.add("RUNTIME_ENV_AGENT_PORT_NAME", "runtime_env_agent_port")?;
    m.add("METRICS_AGENT_PORT_NAME", "metrics_agent_port")?;
    m.add("METRICS_EXPORT_PORT_NAME", "metrics_export_port")?;
    m.add("DASHBOARD_AGENT_LISTEN_PORT_NAME", "dashboard_agent_listen_port")?;
    m.add("GCS_SERVER_PORT_NAME", "gcs_server_port")?;
    m.add("RAY_INTERNAL_DASHBOARD_NAMESPACE", "_ray_internal_dashboard")?;

    // ─── Name aliases matching the original Cython _raylet module ─
    m.add("ObjectID", m.getattr("PyObjectID")?)?;
    m.add("TaskID", m.getattr("PyTaskID")?)?;
    m.add("ActorID", m.getattr("PyActorID")?)?;
    m.add("JobID", m.getattr("PyJobID")?)?;
    m.add("WorkerID", m.getattr("PyWorkerID")?)?;
    m.add("NodeID", m.getattr("PyNodeID")?)?;
    m.add("PlacementGroupID", m.getattr("PyPlacementGroupID")?)?;
    m.add("ActorClassID", m.getattr("PyActorClassID")?)?;
    m.add("FunctionID", m.getattr("PyFunctionID")?)?;
    m.add("UniqueID", m.getattr("PyUniqueID")?)?;
    m.add("ClusterID", m.getattr("PyClusterID")?)?;
    m.add("ObjectRef", m.getattr("PyObjectRef")?)?;
    m.add("Language", m.getattr("PyLanguage")?)?;
    m.add("WorkerType", m.getattr("PyWorkerType")?)?;
    m.add("GcsClient", m.getattr("PyGcsClient")?)?;

    let language = m.getattr("PyLanguage")?;
    language.setattr("PYTHON", language.getattr("Python")?)?;
    language.setattr("JAVA", language.getattr("Java")?)?;
    language.setattr("CPP", language.getattr("Cpp")?)?;

    let worker_type = m.getattr("PyWorkerType")?;
    worker_type.setattr("WORKER", worker_type.getattr("Worker")?)?;
    worker_type.setattr("DRIVER", worker_type.getattr("Driver")?)?;
    worker_type.setattr("SPILL_WORKER", worker_type.getattr("SpillWorker")?)?;
    worker_type.setattr("RESTORE_WORKER", worker_type.getattr("RestoreWorker")?)?;

    Ok(())
}
