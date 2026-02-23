# raylet-rs

`raylet-rs` hosts the Rust rewrite of the Ray raylet. The crate is
part of the top-level Cargo workspace so it can be built and tested in
isolation while FFI hooks are wired into the existing C++ runtime.

## Building

```
cargo build -p raylet-rs
```

This produces both the binary (`raylet-rs`) and a `cdylib` that can be
linked from C++ via the exposed `raylet_entrypoint` shim. During the
build the `cxx` bridge regenerates `src/ray/raylet/scheduling/
rust_scheduler_ffi.h`, which is the header C++ callers should include.

## Testing

```
cargo test -p raylet-rs
```

In addition to the Rust unit tests this command compiles and executes a
small C++ smoke binary (`tests/cpp_scheduler_smoke.rs`) to verify that
`libraylet_rs` links cleanly via the generated header. Ensure a `c++`
compiler is available on your `PATH` when running the tests.
