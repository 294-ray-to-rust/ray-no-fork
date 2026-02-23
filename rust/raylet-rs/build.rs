use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let bridge_src = Path::new("src/scheduler.rs");

    cxx_build::bridge(bridge_src)
        .flag_if_supported("-std=c++17")
        .compile("raylet_scheduler_ffi");

    println!("cargo:rerun-if-changed={}", bridge_src.display());

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let repo_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("failed to resolve repo root from manifest dir");

    let header_candidates = vec![
        out_dir
            .join("cxxbridge")
            .join("include")
            .join("raylet-rs")
            .join("src")
            .join("scheduler.rs.h"),
        out_dir
            .join("cxxbridge")
            .join("rust")
            .join("raylet_rs")
            .join("src")
            .join("scheduler.rs.h"),
    ];

    let generated_header = header_candidates
        .iter()
        .find(|path| path.exists())
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "Generated header missing. Checked: {}",
                header_candidates
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        });

    let header_destination = repo_root
        .join("src")
        .join("ray")
        .join("raylet")
        .join("scheduling")
        .join("rust_scheduler_ffi.h");

    if let Some(parent) = header_destination.parent() {
        fs::create_dir_all(parent).expect("failed to ensure header directory");
    }

    fs::copy(&generated_header, &header_destination)
        .unwrap_or_else(|err| panic!("failed to copy generated header: {}", err));
}
