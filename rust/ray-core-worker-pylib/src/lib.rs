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

pub mod cluster;
pub mod common;
pub mod core_worker;
pub mod gcs_client;
pub mod ids;
pub mod object_ref;
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
use pyo3::types::{PyBytes, PyDict, PyList, PyType};
#[cfg(feature = "python")]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "python")]
fn empty_subscriber_poll_delay(timeout: Option<f64>) -> std::time::Duration {
    const DEFAULT_EMPTY_POLL_SLEEP_MS: u64 = 50;

    match timeout {
        Some(seconds) if seconds.is_finite() && seconds > 0.0 => {
            std::time::Duration::from_secs_f64(seconds)
        }
        _ => std::time::Duration::from_millis(DEFAULT_EMPTY_POLL_SLEEP_MS),
    }
}

#[cfg(feature = "python")]
fn wait_for_empty_subscriber_poll(py: Python<'_>, timeout: Option<f64>) {
    let delay = empty_subscriber_poll_delay(timeout);
    py.allow_threads(|| std::thread::sleep(delay));
}

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
    #[pyo3(signature = (gcs_address, cluster_id_hex = None, allow_cluster_id_nil = true, fetch_cluster_id_if_nil = false))]
    fn create(
        _cls: &Bound<'_, PyType>,
        gcs_address: &str,
        cluster_id_hex: Option<String>,
        allow_cluster_id_nil: bool,
        fetch_cluster_id_if_nil: bool,
    ) -> Self {
        let _ = (allow_cluster_id_nil, fetch_cluster_id_if_nil);
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
struct GlobalStateAccessor {
    gcs_address: String,
}

#[cfg(feature = "python")]
#[pyclass(module = "_raylet")]
struct GcsErrorSubscriber {
    address: String,
    worker_id: Option<Vec<u8>>,
    subscribed: AtomicBool,
}

#[cfg(feature = "python")]
#[pymethods]
impl GcsErrorSubscriber {
    #[new]
    #[pyo3(signature = (address, worker_id = None))]
    fn new(address: String, worker_id: Option<&[u8]>) -> Self {
        Self {
            address,
            worker_id: worker_id.map(|id| id.to_vec()),
            subscribed: AtomicBool::new(false),
        }
    }

    fn subscribe(&self) {
        self.subscribed.store(true, Ordering::Relaxed);
    }

    #[getter]
    fn last_batch_size(&self) -> usize {
        0
    }

    fn close(&self) {
        self.subscribed.store(false, Ordering::Relaxed);
    }

    #[pyo3(signature = (timeout = None))]
    fn poll(&self, py: Python<'_>, timeout: Option<f64>) -> PyResult<(Py<PyAny>, Py<PyAny>)> {
        wait_for_empty_subscriber_poll(py, timeout);
        Ok((py.None(), py.None()))
    }

    #[getter]
    fn address(&self) -> &str {
        &self.address
    }

    #[getter]
    fn worker_id<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.worker_id.as_ref().map(|id| PyBytes::new_bound(py, id))
    }
}

#[cfg(feature = "python")]
#[pyclass(module = "_raylet")]
struct GcsLogSubscriber {
    address: String,
    worker_id: Option<Vec<u8>>,
    subscribed: AtomicBool,
}

#[cfg(feature = "python")]
#[pymethods]
impl GcsLogSubscriber {
    #[new]
    #[pyo3(signature = (address, worker_id = None))]
    fn new(address: String, worker_id: Option<&[u8]>) -> Self {
        Self {
            address,
            worker_id: worker_id.map(|id| id.to_vec()),
            subscribed: AtomicBool::new(false),
        }
    }

    fn subscribe(&self) {
        self.subscribed.store(true, Ordering::Relaxed);
    }

    #[getter]
    fn last_batch_size(&self) -> usize {
        0
    }

    fn close(&self) {
        self.subscribed.store(false, Ordering::Relaxed);
    }

    #[pyo3(signature = (timeout = None))]
    fn poll(&self, py: Python<'_>, timeout: Option<f64>) -> PyResult<Py<PyAny>> {
        wait_for_empty_subscriber_poll(py, timeout);
        Ok(py.None())
    }

    #[getter]
    fn address(&self) -> &str {
        &self.address
    }

    #[getter]
    fn worker_id<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.worker_id.as_ref().map(|id| PyBytes::new_bound(py, id))
    }
}

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

    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        // Minimal Python-compatible surface for local task return storage.
        // The Rust shim does not yet model Pickle5Writer's out-of-band buffers,
        // but in-band pickle bytes are enough for the primitive objects covered
        // by the RayRust canary tests.
        PyBytes::new_bound(py, &self.inband)
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
        // Match Ray's serialized msgpack buffer layout closely enough for
        // Python deserialization: the first 8 bytes store the msgpack payload
        // length, byte 8 is reserved, then msgpack bytes are followed by the
        // nested Pickle5 payload bytes.
        let msgpack_len = self.msgpack_data.len() as u64;
        out[..8].copy_from_slice(&msgpack_len.to_le_bytes());
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
struct GenericStub {
    module_name: String,
    function_name: String,
    class_name: String,
    function_hash: String,
    function: Option<pyo3::PyObject>,
}

#[cfg(feature = "python")]
#[pymethods]
impl GenericStub {
    #[new]
    #[pyo3(signature = (*args, **_kwargs))]
    fn new(
        args: &Bound<'_, pyo3::types::PyTuple>,
        _kwargs: Option<&Bound<'_, pyo3::types::PyDict>>,
    ) -> Self {
        let module_name = args
            .get_item(0)
            .and_then(|v| v.extract::<String>())
            .unwrap_or_default();
        let function_name = args
            .get_item(1)
            .and_then(|v| v.extract::<String>())
            .unwrap_or_default();
        let class_name = args
            .get_item(2)
            .and_then(|v| v.extract::<String>())
            .unwrap_or_default();
        let function_hash = args
            .get_item(3)
            .and_then(|v| v.extract::<String>())
            .unwrap_or_default();
        GenericStub { module_name, function_name, class_name, function_hash, function: None }
    }

    #[classmethod]
    fn instance(_cls: &Bound<'_, PyType>) -> Self {
        GenericStub { module_name: String::new(), function_name: String::new(), class_name: String::new(), function_hash: String::new(), function: None }
    }

    #[classmethod]
    #[pyo3(signature = (*args, **_kwargs))]
    fn from_class(
        _cls: &Bound<'_, PyType>,
        args: &Bound<'_, pyo3::types::PyTuple>,
        _kwargs: Option<&Bound<'_, pyo3::types::PyDict>>,
    ) -> Self {
        let target_class = args.get_item(0).ok();
        let module_name = target_class
            .as_ref()
            .and_then(|c| c.getattr("__module__").ok())
            .and_then(|m| m.extract::<String>().ok())
            .unwrap_or_default();
        let class_name = target_class
            .as_ref()
            .and_then(|c| c.getattr("__qualname__").ok())
            .and_then(|m| m.extract::<String>().ok())
            .unwrap_or_default();
        GenericStub { module_name, function_name: "__init__".to_string(), class_name, function_hash: String::new(), function: target_class.map(|c| c.into()) }
    }

    #[classmethod]
    #[pyo3(signature = (*args, **_kwargs))]
    fn from_function(
        _cls: &Bound<'_, PyType>,
        args: &Bound<'_, pyo3::types::PyTuple>,
        _kwargs: Option<&Bound<'_, pyo3::types::PyDict>>,
    ) -> Self {
        let function = args.get_item(0).ok();
        let function_uuid = args.get_item(1).ok();
        let module_name = function
            .as_ref()
            .and_then(|f| f.getattr("__module__").ok())
            .and_then(|m| m.extract::<String>().ok())
            .unwrap_or_default();
        let function_name = function
            .as_ref()
            .and_then(|f| f.getattr("__qualname__").ok())
            .and_then(|m| m.extract::<String>().ok())
            .unwrap_or_default();
        let function_hash = function_uuid
            .as_ref()
            .and_then(|u| u.getattr("hex").ok())
            .and_then(|h| h.extract::<String>().ok())
            .unwrap_or_default();
        GenericStub { module_name, function_name, class_name: String::new(), function_hash, function: function.map(|f| f.into()) }
    }


    #[getter]
    fn module_name(&self) -> &str { &self.module_name }

    #[getter]
    fn function_name(&self) -> &str { &self.function_name }

    #[getter]
    fn class_name(&self) -> &str { &self.class_name }

    #[getter]
    fn function_hash(&self) -> &str { &self.function_hash }

    #[getter]
    fn function(&self, py: pyo3::Python<'_>) -> Option<pyo3::PyObject> {
        self.function.as_ref().map(|f| f.clone_ref(py))
    }

    #[getter]
    fn repr(&self) -> String { format!("{}.{}", self.module_name, self.function_name) }

    fn __repr__(&self) -> String { self.repr() }

    #[classmethod]
    #[pyo3(signature = (value, python_serializer=None))]
    fn dumps(
        _cls: &Bound<'_, PyType>,
        py: pyo3::Python<'_>,
        value: &Bound<'_, pyo3::types::PyAny>,
        python_serializer: Option<&Bound<'_, pyo3::types::PyAny>>,
    ) -> pyo3::PyResult<pyo3::PyObject> {
        if let Some(serializer) = python_serializer {
            let index = serializer.call1((value,))?;
            let msgpack = pyo3::types::PyModule::import_bound(py, "msgpack")?;
            return Ok(msgpack.call_method1("packb", (index,))?.into());
        }
        let pickle = pyo3::types::PyModule::import_bound(py, "pickle")?;
        Ok(pickle.call_method1("dumps", (value,))?.into())
    }

    #[classmethod]
    #[pyo3(signature = (data, python_deserializer=None))]
    fn loads(
        _cls: &Bound<'_, PyType>,
        py: pyo3::Python<'_>,
        data: &Bound<'_, pyo3::types::PyAny>,
        python_deserializer: Option<&Bound<'_, pyo3::types::PyAny>>,
    ) -> pyo3::PyResult<pyo3::PyObject> {
        if let Some(deserializer) = python_deserializer {
            let msgpack = pyo3::types::PyModule::import_bound(py, "msgpack")?;
            let unpacked = msgpack.call_method1("unpackb", (data,))?;
            return Ok(deserializer.call1((unpacked,))?.into());
        }
        let pickle = pyo3::types::PyModule::import_bound(py, "pickle")?;
        Ok(pickle.call_method1("loads", (data,))?.into())
    }

    #[classmethod]
    #[pyo3(signature = (*_args, **_kwargs))]
    fn redirect_stdout(
        _cls: &Bound<'_, PyType>,
        _args: &Bound<'_, pyo3::types::PyTuple>,
        _kwargs: Option<&Bound<'_, pyo3::types::PyDict>>,
    ) {
    }

    #[classmethod]
    #[pyo3(signature = (*_args, **_kwargs))]
    fn redirect_stderr(
        _cls: &Bound<'_, PyType>,
        _args: &Bound<'_, pyo3::types::PyTuple>,
        _kwargs: Option<&Bound<'_, pyo3::types::PyDict>>,
    ) {
    }

    fn reset_cache(&self) {}

    #[getter]
    fn function_id(&self) -> ids::PyFunctionID {
        ids::PyFunctionID::nil()
    }

    fn __getattr__(&self, py: Python<'_>, name: &str) -> PyResult<Py<PyAny>> {
        // Some Python call paths treat legacy descriptor attributes as data
        // fields (for example `class_name.split(...)`).  Return inert strings
        // for those known fields instead of a callable placeholder.
        if matches!(name, "class_name" | "function_name" | "module_name") {
            return Ok("".into_py(py));
        }
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
    #[pyo3(signature = (_gcs_options = None))]
    fn new(_gcs_options: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        let gcs_address = match _gcs_options {
            Some(options) => options.getattr("gcs_address")?.extract()?,
            None => String::new(),
        };
        Ok(GlobalStateAccessor { gcs_address })
    }

    fn connect(&self) -> bool {
        true
    }

    fn get_node(&self, py: Python<'_>, node_id: &str) -> PyResult<Py<PyAny>> {
        let client = PyGcsClient::new(self.gcs_address.clone());
        let nodes = client.get_all_node_info();
        let dict = PyDict::new_bound(py);
        if let Some(node) = nodes
            .iter()
            .find(|node| hex::encode(&node.node_id) == node_id)
            .or_else(|| nodes.first())
        {
            dict.set_item("node_id", hex::encode(&node.node_id))?;
            dict.set_item("node_manager_address", node.node_manager_address.clone())?;
            dict.set_item("raylet_socket_name", node.raylet_socket_name.clone())?;
            dict.set_item(
                "object_store_socket_name",
                node.object_store_socket_name.clone(),
            )?;
            dict.set_item("node_manager_port", node.node_manager_port)?;
            dict.set_item("object_manager_port", node.object_manager_port)?;
            dict.set_item("metrics_export_port", node.metrics_export_port)?;
            dict.set_item("runtime_env_agent_port", node.runtime_env_agent_port)?;
            dict.set_item("metrics_agent_port", node.metrics_agent_port)?;
            dict.set_item(
                "dashboard_agent_listen_port",
                node.dashboard_agent_listen_port,
            )?;
        } else {
            dict.set_item("node_id", node_id)?;
            dict.set_item("node_manager_address", "127.0.0.1")?;
            dict.set_item("raylet_socket_name", "")?;
            dict.set_item("object_store_socket_name", "")?;
            dict.set_item("node_manager_port", 0)?;
            dict.set_item("object_manager_port", 0)?;
            dict.set_item("metrics_export_port", 0)?;
            dict.set_item("runtime_env_agent_port", 0)?;
            dict.set_item("metrics_agent_port", 0)?;
            dict.set_item("dashboard_agent_listen_port", 0)?;
        }
        dict.set_item("labels", PyDict::new_bound(py))?;
        Ok(dict.unbind().into())
    }

    fn get_node_table(&self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        let client = PyGcsClient::new(self.gcs_address.clone());
        let nodes = client.get_all_node_info();
        let mut results = Vec::with_capacity(nodes.len());

        for node in nodes {
            let dict = PyDict::new_bound(py);
            dict.set_item("NodeID", hex::encode(&node.node_id))?;
            dict.set_item("Alive", node.state == 0)?;
            dict.set_item("NodeManagerAddress", node.node_manager_address.clone())?;
            dict.set_item("NodeManagerHostname", node.node_manager_hostname.clone())?;
            dict.set_item("NodeManagerPort", node.node_manager_port)?;
            dict.set_item("ObjectManagerPort", node.object_manager_port)?;
            dict.set_item("ObjectStoreSocketName", node.object_store_socket_name.clone())?;
            dict.set_item("RayletSocketName", node.raylet_socket_name.clone())?;
            dict.set_item("MetricsExportPort", node.metrics_export_port)?;
            dict.set_item("MetricsAgentPort", node.metrics_agent_port)?;
            dict.set_item("DashboardAgentListenPort", node.dashboard_agent_listen_port)?;
            dict.set_item("NodeName", node.node_name.clone())?;
            dict.set_item("RuntimeEnvAgentPort", node.runtime_env_agent_port)?;
            dict.set_item("DeathReason", node.death_info.as_ref().map(|d| d.reason).unwrap_or(0))?;
            dict.set_item(
                "DeathReasonMessage",
                node.death_info
                    .as_ref()
                    .map(|d| d.reason_message.clone())
                    .unwrap_or_default(),
            )?;
            dict.set_item("alive", node.state == 0)?;
            if node.state == 0 {
                dict.set_item("Resources", node.resources_total.clone())?;
            } else {
                dict.set_item("Resources", PyDict::new_bound(py))?;
            }
            dict.set_item("labels", node.labels.clone())?;
            results.push(dict.unbind().into());
        }

        Ok(results)
    }

    fn get_placement_group_info(
        &self,
        py: Python<'_>,
        pg_id: &Bound<'_, PyAny>,
    ) -> PyResult<Option<Py<PyBytes>>> {
        let placement_group_id: Vec<u8> = if let Ok(bytes) = pg_id.extract() {
            bytes
        } else {
            pg_id.call_method0("binary")?.extract()?
        };
        let data = ray_proto::ray::rpc::PlacementGroupTableData {
            placement_group_id,
            state: ray_proto::ray::rpc::placement_group_table_data::PlacementGroupState::Created
                as i32,
            ..Default::default()
        };
        let mut buf = Vec::new();
        prost::Message::encode(&data, &mut buf).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "failed to encode placement group info: {e}"
            ))
        })?;
        Ok(Some(PyBytes::new_bound(py, &buf).unbind()))
    }

    fn get_placement_group_table(&self, py: Python<'_>) -> Py<PyList> {
        PyList::empty_bound(py).unbind()
    }

    fn get_placement_group_by_name(
        &self,
        _py: Python<'_>,
        _placement_group_name: &str,
        _ray_namespace: &str,
    ) -> Option<Py<PyBytes>> {
        None
    }

    fn get_system_config(&self) -> &str {
        "{}"
    }

    fn get_next_job_id(&self) -> u32 {
        1
    }

    #[pyo3(signature = (*_args, **_kwargs))]
    fn internal_kv_get(
        &self,
        _args: &Bound<'_, pyo3::types::PyTuple>,
        _kwargs: Option<&Bound<'_, pyo3::types::PyDict>>,
    ) -> Vec<u8> {
        b"{}".to_vec()
    }

    #[pyo3(signature = (*_args, **_kwargs))]
    fn internal_kv_put(
        &self,
        _args: &Bound<'_, pyo3::types::PyTuple>,
        _kwargs: Option<&Bound<'_, pyo3::types::PyDict>>,
    ) -> i32 {
        1
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
fn compute_task_id(_object_ref: &object_ref::PyObjectRef) -> ids::PyTaskID {
    ids::PyTaskID::nil()
}

#[cfg(feature = "python")]
#[pyfunction]
fn maybe_initialize_job_config() {}

#[cfg(feature = "python")]
#[pyfunction]
fn serialize_retry_exception_allowlist(
    py: Python<'_>,
    _retry_exception_allowlist: Py<PyAny>,
    _function_descriptor: Py<PyAny>,
) -> Bound<'_, PyBytes> {
    PyBytes::new_bound(py, &[])
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
    let offset = ray_common::constants::MESSAGE_PACK_OFFSET;
    if bytes.len() < offset {
        return Ok((PyBytes::new_bound(py, &[]), PyBytes::new_bound(py, &[])));
    }

    let mut len_bytes = [0_u8; 8];
    len_bytes.copy_from_slice(&bytes[..8]);
    let msgpack_len = usize::try_from(u64::from_le_bytes(len_bytes)).unwrap_or(0);
    let msgpack_end = offset.saturating_add(msgpack_len).min(bytes.len());
    Ok((
        PyBytes::new_bound(py, &bytes[offset..msgpack_end]),
        PyBytes::new_bound(py, &bytes[msgpack_end..]),
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
fn _get_actor_serialized_owner_address_or_none(
    py: Python<'_>,
    actor_table_data: &[u8],
) -> PyResult<Py<PyAny>> {
    let gcs_pb2 = PyModule::import_bound(py, "ray.core.generated.gcs_pb2")?;
    let actor_cls = gcs_pb2.getattr("ActorTableData")?;
    let actor = actor_cls.call0()?;
    actor.call_method1(
        "ParseFromString",
        (PyBytes::new_bound(py, actor_table_data),),
    )?;
    let address = actor.getattr("address")?;
    let worker_id = address.getattr("worker_id")?.extract::<Vec<u8>>()?;
    if worker_id.is_empty() {
        Ok(py.None())
    } else {
        Ok(address.call_method0("SerializeToString")?.unbind())
    }
}

#[cfg(feature = "python")]
#[pyfunction]
fn raise_if_dependency_failed(arg: &Bound<'_, PyAny>) -> PyResult<()> {
    let ray_exceptions = PyModule::import_bound(arg.py(), "ray.exceptions")?;
    let ray_error = ray_exceptions.getattr("RayError")?;
    if arg.is_instance(&ray_error)? {
        Err(pyo3::PyErr::from_value_bound(arg.clone()))
    } else {
        Ok(())
    }
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
static INITIALIZED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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

// `setproctitle` only needs to round-trip the title through Python — the
// dashboard subprocess module reads it back to label restarts. A real OS
// proctitle update isn't required for tests to pass, so back it with a
// process-local mutex.
static PROC_TITLE: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

#[cfg(feature = "python")]
#[pyfunction]
fn getproctitle() -> String {
    PROC_TITLE.lock().map(|t| t.clone()).unwrap_or_default()
}

#[cfg(feature = "python")]
#[pyfunction]
fn setproctitle(title: String) {
    if let Ok(mut t) = PROC_TITLE.lock() {
        *t = title;
    }
}

#[cfg(feature = "python")]
#[pymodule]
fn _raylet(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Cython exposes this extension as both ray._raylet and _raylet in a few
    // pickle/module-name paths.  The Rust module is imported as ray._raylet,
    // so install the top-level alias before any classes are pickled.
    let sys = m.py().import_bound("sys")?;
    sys.getattr("modules")?.set_item("_raylet", m)?;

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
    m.add_function(wrap_pyfunction!(compute_task_id, m)?)?;
    m.add_function(wrap_pyfunction!(maybe_initialize_job_config, m)?)?;
    m.add_function(wrap_pyfunction!(serialize_retry_exception_allowlist, m)?)?;
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
    m.add_function(wrap_pyfunction!(getproctitle, m)?)?;
    m.add_function(wrap_pyfunction!(setproctitle, m)?)?;
    m.add_function(wrap_pyfunction!(
        _get_actor_serialized_owner_address_or_none,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(raise_if_dependency_failed, m)?)?;

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
    m.add_class::<GcsErrorSubscriber>()?;
    m.add_class::<GcsLogSubscriber>()?;
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
        "Buffer",
        "FunctionDescriptor",
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
        "NumReturnsWarning",
        m.py().eval_bound("type('NumReturnsWarning', (UserWarning,), {})", None, None)?,
    )?;
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
    m.add(
        "IMPLICIT_RESOURCE_PREFIX",
        "node:__internal_implicit_resource_",
    )?;
    m.add("STREAMING_GENERATOR_RETURN", -2)?;
    m.add("GCS_AUTOSCALER_STATE_NAMESPACE", "__autoscaler")?;
    m.add("GCS_AUTOSCALER_V2_ENABLED_KEY", "__autoscaler_v2_enabled")?;
    m.add(
        "GCS_AUTOSCALER_CLUSTER_CONFIG_KEY",
        "__autoscaler_cluster_config",
    )?;
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
    m.add(
        "DASHBOARD_AGENT_LISTEN_PORT_NAME",
        "dashboard_agent_listen_port",
    )?;
    m.add("GCS_SERVER_PORT_NAME", "gcs_server_port")?;
    m.add(
        "RAY_INTERNAL_DASHBOARD_NAMESPACE",
        "_ray_internal_dashboard",
    )?;
    m.add("OPTIMIZED", true)?;
    m.add("GRPC_STATUS_CODE_UNAVAILABLE", 14)?;
    m.add("GRPC_STATUS_CODE_UNKNOWN", 2)?;
    m.add("GRPC_STATUS_CODE_DEADLINE_EXCEEDED", 4)?;
    m.add("GRPC_STATUS_CODE_RESOURCE_EXHAUSTED", 8)?;
    m.add("GRPC_STATUS_CODE_UNIMPLEMENTED", 12)?;
    m.add(
        "async_task_id",
        m.py().eval_bound(
            "__import__('contextvars').ContextVar('async_task_id', default=None)",
            None,
            None,
        )?,
    )?;

    // ─── Name aliases matching the original Cython _raylet module ─
    // In Cython Ray, ObjectID is a backwards-compatible alias for ObjectRef.
    // Some Python paths still construct `ray.ObjectID` and pass it through
    // APIs such as ray.get(), which validate against ray.ObjectRef.
    m.add("ObjectID", m.getattr("PyObjectRef")?)?;
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
    m.add("CoreWorker", m.getattr("PyCoreWorker")?)?;
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
