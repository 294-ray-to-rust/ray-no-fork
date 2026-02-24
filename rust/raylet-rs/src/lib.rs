//! Core library for the Rust-based raylet prototype.
//! Currently contains minimal scaffolding so other components can
//! link against a stable FFI entry point while the implementation
//! is developed.

use std::os::raw::c_int;

pub mod scheduling_ffi;
mod cluster_resource_scheduler;
mod scheduler_ffi;

/// Minimal main entry point for the Rust raylet implementation.
///
/// The function is intentionally simple for now so that later
/// changes can focus on wiring real subsystems without touching
/// the public surface.
pub fn raylet_main() -> Result<(), RayletError> {
    // TODO: boot real subsystems once they are ported to Rust.
    Ok(())
}

/// A lightweight error type so callers can distinguish between
/// success and failure without committing to a specific error stack yet.
#[derive(Debug)]
pub struct RayletError {
    pub message: &'static str,
}

impl std::fmt::Display for RayletError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RayletError {}

/// Extern "C" shim so C++ callers can invoke the Rust raylet logic.
///
/// Returns `0` on success and `-1` on failure until a richer status
/// contract is required.
#[no_mangle]
pub extern "C" fn raylet_entrypoint() -> c_int {
    match raylet_main() {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("raylet_entrypoint failed: {}", err);
            -1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raylet_main_succeeds() {
        assert!(raylet_main().is_ok());
    }

    #[test]
    fn ffi_entrypoint_returns_zero() {
        assert_eq!(raylet_entrypoint(), 0);
    }
}
