use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_HANDLE_ID: AtomicU64 = AtomicU64::new(1);

#[cxx::bridge(namespace = "ray::raylet")]
pub mod ffi {
    extern "Rust" {
        type RayletSchedulerHandle;

        fn raylet_scheduler_create() -> Box<RayletSchedulerHandle>;
        fn raylet_scheduler_destroy(handle: Box<RayletSchedulerHandle>);
        fn raylet_scheduler_update_cluster_view(handle: &RayletSchedulerHandle);
        fn raylet_scheduler_allocate(handle: &RayletSchedulerHandle);
        fn raylet_scheduler_release(handle: &RayletSchedulerHandle);
    }
}

/// Opaque state owned by the Rust scheduler.
///
/// The struct only tracks a unique ID for now so pointer lifecycles can be
/// asserted in debug builds. Scheduling logic will replace this placeholder.
pub struct RayletSchedulerHandle {
    id: u64,
}

impl RayletSchedulerHandle {
    fn new() -> Self {
        let id = NEXT_HANDLE_ID.fetch_add(1, Ordering::Relaxed);
        Self { id }
    }

    fn ensure_live(&self, op: &str) {
        debug_assert!(self.id != 0, "RayletSchedulerHandle used after free during {}", op);
    }
}

pub fn raylet_scheduler_create() -> Box<RayletSchedulerHandle> {
    Box::new(RayletSchedulerHandle::new())
}

pub fn raylet_scheduler_destroy(_handle: Box<RayletSchedulerHandle>) {
    // Dropping the Box is sufficient for now. Real implementation will flush
    // metrics and gracefully tear down background tasks.
}

pub fn raylet_scheduler_update_cluster_view(handle: &RayletSchedulerHandle) {
    handle.ensure_live("update_cluster_view");
}

pub fn raylet_scheduler_allocate(handle: &RayletSchedulerHandle) {
    handle.ensure_live("allocate");
}

pub fn raylet_scheduler_release(handle: &RayletSchedulerHandle) {
    handle.ensure_live("release");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_destroy_handle() {
        let handle = raylet_scheduler_create();
        assert!(handle.id > 0);
        raylet_scheduler_destroy(handle);
    }
}
