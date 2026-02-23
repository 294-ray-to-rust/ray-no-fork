use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("failed to find workspace root from manifest dir")
        .to_path_buf()
}

fn target_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        PathBuf::from(dir)
    } else {
        workspace_root().join("target")
    }
}

fn profile_dir() -> PathBuf {
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    target_dir().join(profile)
}

fn cpp_compiler() -> String {
    std::env::var("CXX").unwrap_or_else(|_| "c++".to_string())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn cpp_binary_can_link_scheduler_handle() {
    let repo_root = workspace_root();
    let header = repo_root
        .join("src")
        .join("ray")
        .join("raylet")
        .join("scheduling")
        .join("rust_scheduler_ffi.h");
    assert!(header.exists(), "expected header at {}", header.display());

    let cpp_src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("cpp")
        .join("scheduler_ffi_smoke.cc");
    assert!(cpp_src.exists(), "missing smoke source {}", cpp_src.display());

    let profile_dir = profile_dir();
    let lib_name = format!(
        "{}raylet_rs{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );
    let lib_path = profile_dir.join(&lib_name);
    assert!(lib_path.exists(), "missing cdylib {}", lib_path.display());
    let bin_path = profile_dir.join(format!(
        "scheduler_ffi_smoke{}",
        std::env::consts::EXE_SUFFIX
    ));

    let mut compile_cmd = Command::new(cpp_compiler());
    let status = compile_cmd
        .arg("-std=c++17")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg(&cpp_src)
        .arg("-o")
        .arg(&bin_path)
        .arg(format!("-I{}", repo_root.join("src").display()))
        .arg(format!("-L{}", profile_dir.display()))
        .arg(format!("-Wl,-rpath,{}", profile_dir.display()))
        .arg("-lraylet_rs")
        .status()
        .expect("failed to spawn C++ compiler");

    assert!(status.success(), "failed to compile C++ smoke binary");

    let status = Command::new(&bin_path)
        .status()
        .expect("failed to execute C++ smoke binary");
    assert!(status.success(), "C++ smoke binary exited with failure");
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
#[test]
fn cpp_smoke_skipped_on_unsupported_platform() {
    eprintln!("Skipping CPP smoke test on unsupported platform");
}
