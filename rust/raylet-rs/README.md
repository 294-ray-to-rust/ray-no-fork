# raylet-rs

`raylet-rs` hosts the Rust rewrite of the Ray raylet. The crate is
part of the top-level Cargo workspace so it can be built and tested in
isolation while FFI hooks are wired into the existing C++ runtime.

## Building

```
cargo build -p raylet-rs
```

This produces both the binary (`raylet-rs`) and a `cdylib` that can be
linked from C++ via the exposed `raylet_entrypoint` shim.

## Testing

```
cargo test -p raylet-rs
```

The current tests are smoke-level to ensure the entry points stay
callable. Expand them as subsystems migrate to Rust.

## Continuous Integration

Pushes and pull requests that touch this crate automatically run the
`raylet-rs` GitHub Actions workflow:

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p raylet-rs`

Keep these checks passing before merging any changes.
