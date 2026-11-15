//! Copy `WebView2Loader.dll` next to the built executable.
//!
//! `webview2-com-sys` links the loader's import library, but the runtime DLL
//! still has to be findable when the process starts. The DLL ships inside the
//! `webview2-com-sys` crate, so it is always available on disk, whichever
//! order the build scripts happen to run in.

use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let Ok(out_dir) = std::env::var("OUT_DIR").map(PathBuf::from) else {
        return;
    };

    // OUT_DIR is <target>/<profile>/build/<pkg>-<hash>/out
    let Some(build_dir) = out_dir.ancestors().nth(2).map(Path::to_path_buf) else {
        return;
    };
    let Some(profile_dir) = out_dir.ancestors().nth(3).map(Path::to_path_buf) else {
        return;
    };

    let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86") => "x86",
        _ => "x64",
    };

    let mut candidates: Vec<PathBuf> = Vec::new();

    // The crate source always has the loader.
    if let Ok(registry) = std::env::var("CARGO_HOME") {
        let registry = PathBuf::from(registry);
        let sources = if registry.join("registry/src").is_dir() {
            registry.join("registry/src")
        } else {
            registry.join("registry/src")
        };
        if let Ok(indexes) = std::fs::read_dir(&sources) {
            for index in indexes.flatten() {
                let packages = index.path().join("webview2-com-sys-");
                let Ok(entries) = std::fs::read_dir(&packages) else {
                    continue;
                };
                for entry in entries.flatten() {
                    candidates.push(entry.path().join(arch).join("WebView2Loader.dll"));
                }
            }
        }
    }

    // The build output of the sys crate, if it has already run.
    if let Ok(entries) = std::fs::read_dir(&build_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("webview2-com-sys-") {
                candidates.push(entry.path().join("out").join(arch).join("WebView2Loader.dll"));
            }
        }
    }

    let Some(source) = candidates.into_iter().find(|path| path.exists()) else {
        println!("cargo:warning=WebView2Loader.dll not found; the web host may fail to start");
        return;
    };

    let destination = profile_dir.join("WebView2Loader.dll");
    match std::fs::copy(&source, &destination) {
        Ok(_) => println!("cargo:warning=copied WebView2Loader.dll from {}", source.display()),
        Err(err) => println!("cargo:warning=could not copy WebView2Loader.dll: {err}"),
    }
}
