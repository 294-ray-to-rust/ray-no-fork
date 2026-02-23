use std::fmt;

const HANDLE_MAGIC: u64 = 0x5259_4c54_5343_4844;

#[cxx::bridge(namespace = "ray::raylet::scheduling")]
mod ffi {
    extern "Rust" {
        type RayletSchedulerHandle;

        fn raylet_rs_scheduler_create() -> Box<RayletSchedulerHandle>;
        fn raylet_rs_scheduler_destroy(handle: Box<RayletSchedulerHandle>);
        fn raylet_rs_scheduler_update_cluster_view(handle: &mut RayletSchedulerHandle);
        fn raylet_rs_scheduler_allocate(handle: &mut RayletSchedulerHandle) -> bool;
        fn raylet_rs_scheduler_release(handle: &mut RayletSchedulerHandle);
    }
}

struct RayletSchedulerHandle {
    magic: u64,
    alive: bool,
}

impl fmt::Debug for RayletSchedulerHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RayletSchedulerHandle")
            .field("magic", &self.magic)
            .field("alive", &self.alive)
            .finish()
    }
}

fn raylet_rs_scheduler_create() -> Box<RayletSchedulerHandle> {
    Box::new(RayletSchedulerHandle {
        magic: HANDLE_MAGIC,
        alive: true,
    })
}

fn raylet_rs_scheduler_destroy(mut handle: Box<RayletSchedulerHandle>) {
    validate_handle(&handle);
    handle.alive = false;
}

fn raylet_rs_scheduler_update_cluster_view(handle: &mut RayletSchedulerHandle) {
    validate_handle(handle);
}

fn raylet_rs_scheduler_allocate(handle: &mut RayletSchedulerHandle) -> bool {
    validate_handle(handle);
    false
}

fn raylet_rs_scheduler_release(handle: &mut RayletSchedulerHandle) {
    validate_handle(handle);
}

fn validate_handle(handle: &RayletSchedulerHandle) {
    assert_eq!(
        handle.magic, HANDLE_MAGIC,
        "raylet scheduler handle corrupted"
    );
    assert!(handle.alive, "raylet scheduler handle already destroyed");
}
