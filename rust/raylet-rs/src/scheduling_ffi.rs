use std::collections::HashMap;
use std::os::raw::c_char;
use std::ptr;
use std::slice;
use std::str;

use crate::scheduling::local_resource_manager::{LocalResourceManager, WorkFootprint};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FfiError {
    NullPointer(&'static str),
    InvalidUtf8(&'static str),
}

impl std::fmt::Display for FfiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FfiError::NullPointer(context) => write!(f, "null pointer in {}", context),
            FfiError::InvalidUtf8(context) => write!(f, "invalid utf8 in {}", context),
        }
    }
}

impl std::error::Error for FfiError {}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RayletStr {
    pub data: *const c_char,
    pub len: usize,
}

impl RayletStr {
    pub unsafe fn as_bytes<'a>(&self) -> Result<&'a [u8], FfiError> {
        if self.data.is_null() {
            if self.len == 0 {
                return Ok(&[]);
            }
            return Err(FfiError::NullPointer("RayletStr"));
        }
        Ok(slice::from_raw_parts(self.data as *const u8, self.len))
    }

    pub unsafe fn as_str<'a>(&self) -> Result<&'a str, FfiError> {
        let bytes = self.as_bytes()?;
        str::from_utf8(bytes).map_err(|_| FfiError::InvalidUtf8("RayletStr"))
    }

    pub unsafe fn to_string(&self) -> Result<String, FfiError> {
        Ok(self.as_str()?.to_owned())
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RayletStrArray {
    pub entries: *const RayletStr,
    pub len: usize,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RayletResourceEntry {
    pub name: RayletStr,
    pub value: f64,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RayletResourceArray {
    pub entries: *const RayletResourceEntry,
    pub len: usize,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RayletLabelEntry {
    pub key: RayletStr,
    pub value: RayletStr,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RayletLabelArray {
    pub entries: *const RayletLabelEntry,
    pub len: usize,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RayletLabelSelectorOp {
    Unspecified = 0,
    In = 1,
    NotIn = 2,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RayletLabelConstraint {
    pub key: RayletStr,
    pub op: RayletLabelSelectorOp,
    pub values: RayletStrArray,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RayletLabelSelector {
    pub constraints: *const RayletLabelConstraint,
    pub len: usize,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RayletResourceRequest {
    pub resources: RayletResourceArray,
    pub requires_object_store_memory: u8,
    pub label_selector: RayletLabelSelector,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RayletNodeResources {
    pub total: RayletResourceArray,
    pub available: RayletResourceArray,
    pub load: RayletResourceArray,
    pub normal_task_resources: RayletResourceArray,
    pub labels: RayletLabelArray,
    pub idle_resource_duration_ms: i64,
    pub is_draining: u8,
    pub draining_deadline_timestamp_ms: i64,
    pub last_resource_update_ms: i64,
    pub latest_resources_normal_task_timestamp: i64,
    pub object_pulls_queued: u8,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RayletNodeResourceView {
    pub node_id: i64,
    pub resources: RayletNodeResources,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RayletNodeResourceViewArray {
    pub entries: *const RayletNodeResourceView,
    pub len: usize,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RayletSchedulingRequest {
    pub request_id: i64,
    pub preferred_node_id: i64,
    pub resource_request: RayletResourceRequest,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RayletSchedulingDecision {
    pub request_id: i64,
    pub selected_node_id: i64,
    pub is_feasible: u8,
    pub is_spillback: u8,
}

#[repr(C)]
pub struct RayletLocalResourceManagerHandle {
    manager: LocalResourceManager,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RayletWorkFootprint {
    NodeWorkers = 1,
    PullingTaskArguments = 2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelSelectorOperator {
    Unspecified,
    In,
    NotIn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelConstraint {
    pub key: String,
    pub op: LabelSelectorOperator,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelSelector {
    pub constraints: Vec<LabelConstraint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceRequest {
    pub resources: HashMap<String, f64>,
    pub requires_object_store_memory: bool,
    pub label_selector: LabelSelector,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeResources {
    pub total: HashMap<String, f64>,
    pub available: HashMap<String, f64>,
    pub load: HashMap<String, f64>,
    pub normal_task_resources: HashMap<String, f64>,
    pub labels: HashMap<String, String>,
    pub idle_resource_duration_ms: i64,
    pub is_draining: bool,
    pub draining_deadline_timestamp_ms: i64,
    pub last_resource_update_ms: i64,
    pub latest_resources_normal_task_timestamp: i64,
    pub object_pulls_queued: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeResourceView {
    pub node_id: i64,
    pub resources: NodeResources,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchedulingRequest {
    pub request_id: i64,
    pub preferred_node_id: i64,
    pub resource_request: ResourceRequest,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchedulingDecision {
    pub request_id: i64,
    pub selected_node_id: i64,
    pub is_feasible: bool,
    pub is_spillback: bool,
}

impl RayletLabelSelectorOp {
    fn to_rust(self) -> LabelSelectorOperator {
        match self {
            RayletLabelSelectorOp::Unspecified => LabelSelectorOperator::Unspecified,
            RayletLabelSelectorOp::In => LabelSelectorOperator::In,
            RayletLabelSelectorOp::NotIn => LabelSelectorOperator::NotIn,
        }
    }
}

unsafe fn slice_from_raw<'a, T>(ptr: *const T, len: usize, context: &'static str) -> Result<&'a [T], FfiError> {
    if ptr.is_null() {
        if len == 0 {
            return Ok(&[]);
        }
        return Err(FfiError::NullPointer(context));
    }
    Ok(slice::from_raw_parts(ptr, len))
}

impl RayletResourceArray {
    pub unsafe fn to_map(&self) -> Result<HashMap<String, f64>, FfiError> {
        let entries = slice_from_raw(self.entries, self.len, "RayletResourceArray")?;
        let mut map = HashMap::with_capacity(entries.len());
        for entry in entries {
            let name = entry.name.to_string()?;
            map.insert(name, entry.value);
        }
        Ok(map)
    }
}

impl RayletLabelArray {
    pub unsafe fn to_map(&self) -> Result<HashMap<String, String>, FfiError> {
        let entries = slice_from_raw(self.entries, self.len, "RayletLabelArray")?;
        let mut map = HashMap::with_capacity(entries.len());
        for entry in entries {
            let key = entry.key.to_string()?;
            let value = entry.value.to_string()?;
            map.insert(key, value);
        }
        Ok(map)
    }
}

impl RayletLabelSelector {
    pub unsafe fn to_rust(&self) -> Result<LabelSelector, FfiError> {
        let constraints = slice_from_raw(self.constraints, self.len, "RayletLabelSelector")?;
        let mut out = Vec::with_capacity(constraints.len());
        for constraint in constraints {
            let key = constraint.key.to_string()?;
            let values = constraint.values.to_vec()?;
            let values = values
                .into_iter()
                .map(|entry| entry.to_string())
                .collect::<Result<Vec<_>, _>>()?;
            out.push(LabelConstraint {
                key,
                op: constraint.op.to_rust(),
                values,
            });
        }
        Ok(LabelSelector { constraints: out })
    }
}

impl RayletStrArray {
    pub unsafe fn to_vec(&self) -> Result<Vec<RayletStr>, FfiError> {
        let entries = slice_from_raw(self.entries, self.len, "RayletStrArray")?;
        Ok(entries.to_vec())
    }
}

impl RayletResourceRequest {
    pub unsafe fn to_rust(&self) -> Result<ResourceRequest, FfiError> {
        Ok(ResourceRequest {
            resources: self.resources.to_map()?,
            requires_object_store_memory: self.requires_object_store_memory != 0,
            label_selector: self.label_selector.to_rust()?,
        })
    }
}

impl RayletNodeResources {
    pub unsafe fn to_rust(&self) -> Result<NodeResources, FfiError> {
        Ok(NodeResources {
            total: self.total.to_map()?,
            available: self.available.to_map()?,
            load: self.load.to_map()?,
            normal_task_resources: self.normal_task_resources.to_map()?,
            labels: self.labels.to_map()?,
            idle_resource_duration_ms: self.idle_resource_duration_ms,
            is_draining: self.is_draining != 0,
            draining_deadline_timestamp_ms: self.draining_deadline_timestamp_ms,
            last_resource_update_ms: self.last_resource_update_ms,
            latest_resources_normal_task_timestamp: self.latest_resources_normal_task_timestamp,
            object_pulls_queued: self.object_pulls_queued != 0,
        })
    }
}

impl RayletNodeResourceView {
    pub unsafe fn to_rust(&self) -> Result<NodeResourceView, FfiError> {
        Ok(NodeResourceView {
            node_id: self.node_id,
            resources: self.resources.to_rust()?,
        })
    }
}

impl RayletSchedulingRequest {
    pub unsafe fn to_rust(&self) -> Result<SchedulingRequest, FfiError> {
        Ok(SchedulingRequest {
            request_id: self.request_id,
            preferred_node_id: self.preferred_node_id,
            resource_request: self.resource_request.to_rust()?,
        })
    }
}

impl RayletSchedulingDecision {
    pub fn to_rust(&self) -> SchedulingDecision {
        SchedulingDecision {
            request_id: self.request_id,
            selected_node_id: self.selected_node_id,
            is_feasible: self.is_feasible != 0,
            is_spillback: self.is_spillback != 0,
        }
    }
}

impl RayletWorkFootprint {
    fn to_rust(self) -> WorkFootprint {
        match self {
            Self::NodeWorkers => WorkFootprint::NodeWorkers,
            Self::PullingTaskArguments => WorkFootprint::PullingTaskArguments,
        }
    }
}

#[no_mangle]
pub extern "C" fn raylet_rs_scheduler_roundtrip(
    request: *const RayletSchedulingRequest,
    decision_out: *mut RayletSchedulingDecision,
) -> u8 {
    if request.is_null() || decision_out.is_null() {
        return 0;
    }
    let request = unsafe { &*request };
    let decision = RayletSchedulingDecision {
        request_id: request.request_id,
        selected_node_id: request.preferred_node_id,
        is_feasible: 1,
        is_spillback: 0,
    };
    unsafe {
        *decision_out = decision;
    }
    1
}

#[no_mangle]
pub extern "C" fn raylet_rs_local_resource_manager_create(
    node_resources: *const RayletNodeResources,
) -> *mut RayletLocalResourceManagerHandle {
    if node_resources.is_null() {
        return ptr::null_mut();
    }

    let node_resources = unsafe { &*node_resources };
    let node_resources = match unsafe { node_resources.to_rust() } {
        Ok(resources) => resources,
        Err(_) => return ptr::null_mut(),
    };

    let handle = RayletLocalResourceManagerHandle {
        manager: LocalResourceManager::new(node_resources),
    };

    Box::into_raw(Box::new(handle))
}

#[no_mangle]
pub extern "C" fn raylet_rs_local_resource_manager_destroy(
    handle: *mut RayletLocalResourceManagerHandle,
) {
    if handle.is_null() {
        return;
    }

    unsafe {
        drop(Box::from_raw(handle));
    }
}

#[no_mangle]
pub extern "C" fn raylet_rs_local_resource_manager_allocate(
    handle: *mut RayletLocalResourceManagerHandle,
    request: *const RayletResourceRequest,
) -> u8 {
    if handle.is_null() || request.is_null() {
        return 0;
    }

    let handle = unsafe { &mut *handle };
    let request = unsafe { &*request };
    let request = match unsafe { request.to_rust() } {
        Ok(request) => request,
        Err(_) => return 0,
    };

    handle.manager.allocate(&request) as u8
}

#[no_mangle]
pub extern "C" fn raylet_rs_local_resource_manager_release(
    handle: *mut RayletLocalResourceManagerHandle,
    resources: *const RayletResourceArray,
) -> u8 {
    if handle.is_null() || resources.is_null() {
        return 0;
    }

    let handle = unsafe { &mut *handle };
    let resources = unsafe { &*resources };
    let resources = match unsafe { resources.to_map() } {
        Ok(resources) => resources,
        Err(_) => return 0,
    };

    handle.manager.release(&resources);
    1
}

#[no_mangle]
pub extern "C" fn raylet_rs_local_resource_manager_get_available(
    handle: *const RayletLocalResourceManagerHandle,
    resource_name: RayletStr,
    available_out: *mut f64,
) -> u8 {
    if handle.is_null() || available_out.is_null() {
        return 0;
    }

    let resource_name = match unsafe { resource_name.as_str() } {
        Ok(name) => name,
        Err(_) => return 0,
    };

    let handle = unsafe { &*handle };
    let Some(available) = handle.manager.get_available(resource_name) else {
        return 0;
    };

    unsafe {
        *available_out = available;
    }
    1
}

#[no_mangle]
pub extern "C" fn raylet_rs_local_resource_manager_add_resource_instances(
    handle: *mut RayletLocalResourceManagerHandle,
    resource_name: RayletStr,
    amount: f64,
) -> u8 {
    if handle.is_null() {
        return 0;
    }

    let resource_name = match unsafe { resource_name.as_str() } {
        Ok(name) => name,
        Err(_) => return 0,
    };

    let handle = unsafe { &mut *handle };
    handle.manager.add_resource_instances(resource_name, amount);
    1
}

#[no_mangle]
pub extern "C" fn raylet_rs_local_resource_manager_subtract_resource_instances(
    handle: *mut RayletLocalResourceManagerHandle,
    resource_name: RayletStr,
    amount: f64,
    allow_going_negative: u8,
    underflow_out: *mut f64,
) -> u8 {
    if handle.is_null() || underflow_out.is_null() {
        return 0;
    }

    let resource_name = match unsafe { resource_name.as_str() } {
        Ok(name) => name,
        Err(_) => return 0,
    };

    let handle = unsafe { &mut *handle };
    let underflow = handle.manager.subtract_resource_instances(
        resource_name,
        amount,
        allow_going_negative != 0,
    );
    unsafe {
        *underflow_out = underflow;
    }

    1
}

#[no_mangle]
pub extern "C" fn raylet_rs_local_resource_manager_mark_footprint_busy(
    handle: *mut RayletLocalResourceManagerHandle,
    footprint: RayletWorkFootprint,
) -> u8 {
    if handle.is_null() {
        return 0;
    }
    let handle = unsafe { &mut *handle };
    handle.manager.mark_footprint_as_busy(footprint.to_rust());
    1
}

#[no_mangle]
pub extern "C" fn raylet_rs_local_resource_manager_maybe_mark_footprint_busy(
    handle: *mut RayletLocalResourceManagerHandle,
    footprint: RayletWorkFootprint,
) -> u8 {
    if handle.is_null() {
        return 0;
    }
    let handle = unsafe { &mut *handle };
    handle
        .manager
        .maybe_mark_footprint_as_busy(footprint.to_rust());
    1
}

#[no_mangle]
pub extern "C" fn raylet_rs_local_resource_manager_mark_footprint_idle(
    handle: *mut RayletLocalResourceManagerHandle,
    footprint: RayletWorkFootprint,
) -> u8 {
    if handle.is_null() {
        return 0;
    }
    let handle = unsafe { &mut *handle };
    handle.manager.mark_footprint_as_idle(footprint.to_rust());
    1
}

#[no_mangle]
pub extern "C" fn raylet_rs_local_resource_manager_is_node_idle(
    handle: *const RayletLocalResourceManagerHandle,
) -> u8 {
    if handle.is_null() {
        return 0;
    }
    let handle = unsafe { &*handle };
    handle.manager.is_local_node_idle() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn resource_array_converts_to_map() {
        let cpu = RayletStr {
            data: b"CPU".as_ptr() as *const c_char,
            len: 3,
        };
        let gpu = RayletStr {
            data: b"GPU".as_ptr() as *const c_char,
            len: 3,
        };
        let entries = [
            RayletResourceEntry { name: cpu, value: 2.0 },
            RayletResourceEntry { name: gpu, value: 1.0 },
        ];
        let array = RayletResourceArray {
            entries: entries.as_ptr(),
            len: entries.len(),
        };
        let map = unsafe { array.to_map() }.expect("map conversion");
        assert_eq!(map.get("CPU"), Some(&2.0));
        assert_eq!(map.get("GPU"), Some(&1.0));
    }

    #[test]
    fn label_selector_converts_to_rust() {
        let key = RayletStr {
            data: b"node".as_ptr() as *const c_char,
            len: 4,
        };
        let value = RayletStr {
            data: b"alpha".as_ptr() as *const c_char,
            len: 5,
        };
        let values = [value];
        let values_array = RayletStrArray {
            entries: values.as_ptr(),
            len: values.len(),
        };
        let constraint = RayletLabelConstraint {
            key,
            op: RayletLabelSelectorOp::In,
            values: values_array,
        };
        let selector = RayletLabelSelector {
            constraints: &constraint as *const RayletLabelConstraint,
            len: 1,
        };
        let rust_selector = unsafe { selector.to_rust() }.expect("selector");
        assert_eq!(rust_selector.constraints.len(), 1);
        let constraint = &rust_selector.constraints[0];
        assert_eq!(constraint.key, "node");
        assert_eq!(constraint.values, vec!["alpha".to_string()]);
    }

    #[test]
    fn scheduling_roundtrip_sets_decision() {
        let resource_request = RayletResourceRequest {
            resources: RayletResourceArray {
                entries: std::ptr::null(),
                len: 0,
            },
            requires_object_store_memory: 0,
            label_selector: RayletLabelSelector {
                constraints: std::ptr::null(),
                len: 0,
            },
        };
        let request = RayletSchedulingRequest {
            request_id: 42,
            preferred_node_id: 7,
            resource_request,
        };
        let mut decision = RayletSchedulingDecision {
            request_id: 0,
            selected_node_id: 0,
            is_feasible: 0,
            is_spillback: 0,
        };
        let ok = raylet_rs_scheduler_roundtrip(&request as *const _, &mut decision as *mut _);
        assert_eq!(ok, 1);
        assert_eq!(decision.request_id, 42);
        assert_eq!(decision.selected_node_id, 7);
        assert_eq!(decision.is_feasible, 1);
        assert_eq!(decision.is_spillback, 0);
    }

    #[test]
    fn ffi_layout_matches_cpp_expectations() {
        assert_eq!(size_of::<RayletStr>(), 16);
        assert_eq!(size_of::<RayletStrArray>(), 16);
        assert_eq!(size_of::<RayletResourceEntry>(), 24);
        assert_eq!(size_of::<RayletLabelEntry>(), 32);
        assert_eq!(size_of::<RayletResourceRequest>(), 40);
        assert_eq!(size_of::<RayletSchedulingRequest>(), 56);
        assert_eq!(size_of::<RayletSchedulingDecision>(), 24);
    }

    #[test]
    fn local_resource_manager_ffi_allocate_release() {
        let cpu = RayletStr {
            data: b"CPU".as_ptr() as *const c_char,
            len: 3,
        };
        let entries = [RayletResourceEntry {
            name: cpu,
            value: 2.0,
        }];
        let resources = RayletResourceArray {
            entries: entries.as_ptr(),
            len: entries.len(),
        };
        let node_resources = RayletNodeResources {
            total: resources,
            available: resources,
            load: RayletResourceArray {
                entries: ptr::null(),
                len: 0,
            },
            normal_task_resources: RayletResourceArray {
                entries: ptr::null(),
                len: 0,
            },
            labels: RayletLabelArray {
                entries: ptr::null(),
                len: 0,
            },
            idle_resource_duration_ms: 0,
            is_draining: 0,
            draining_deadline_timestamp_ms: -1,
            last_resource_update_ms: 0,
            latest_resources_normal_task_timestamp: 0,
            object_pulls_queued: 0,
        };

        let handle = raylet_rs_local_resource_manager_create(&node_resources as *const _);
        assert!(!handle.is_null());

        let request_entries = [RayletResourceEntry {
            name: cpu,
            value: 1.0,
        }];
        let request = RayletResourceRequest {
            resources: RayletResourceArray {
                entries: request_entries.as_ptr(),
                len: request_entries.len(),
            },
            requires_object_store_memory: 0,
            label_selector: RayletLabelSelector {
                constraints: ptr::null(),
                len: 0,
            },
        };
        assert_eq!(raylet_rs_local_resource_manager_allocate(handle, &request), 1);

        let mut available = -1.0;
        assert_eq!(
            raylet_rs_local_resource_manager_get_available(handle, cpu, &mut available),
            1
        );
        assert_eq!(available, 1.0);

        let release_entries = [RayletResourceEntry {
            name: cpu,
            value: 1.0,
        }];
        let release_resources = RayletResourceArray {
            entries: release_entries.as_ptr(),
            len: release_entries.len(),
        };
        assert_eq!(
            raylet_rs_local_resource_manager_release(handle, &release_resources),
            1
        );
        assert_eq!(
            raylet_rs_local_resource_manager_get_available(handle, cpu, &mut available),
            1
        );
        assert_eq!(available, 2.0);

        raylet_rs_local_resource_manager_destroy(handle);
    }
}
