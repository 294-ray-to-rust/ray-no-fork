use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let mut build = cxx_build::bridge("src/scheduler_ffi.rs");
    build.flag_if_supported("-std=c++17");
    build.compile("raylet_rs_scheduler_ffi");

    println!("cargo:rerun-if-changed=src/scheduler_ffi.rs");
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let header_src = generated_header_path(&out_dir);

    let header_out = match env::var("RAYLET_RS_HEADER_OUT") {
        Ok(value) if value.is_empty() || value == "skip" => return,
        Ok(value) => PathBuf::from(value),
        Err(_) => default_header_path(),
    };

    if let Some(parent) = header_out.parent() {
        fs::create_dir_all(parent).expect("failed to create header output directory");
    }

    fs::copy(&header_src, &header_out)
        .expect("failed to copy generated scheduler FFI header");
}

fn default_header_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let repo_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("failed to locate repo root");
    repo_root
        .join("src")
        .join("ray")
        .join("raylet")
        .join("scheduling")
        .join("rust_scheduler_ffi.h")
}

fn generated_header_path(out_dir: &Path) -> PathBuf {
    let include_dir = out_dir.join("cxxbridge").join("include");
    let underscore = include_dir
        .join("raylet_rs")
        .join("src")
        .join("scheduler_ffi.rs.h");
    if underscore.exists() {
        return underscore;
    }
    include_dir
        .join("raylet-rs")
        .join("src")
        .join("scheduler_ffi.rs.h")
}
