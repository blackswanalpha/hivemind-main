fn main() {
    // Embed an rpath so the produced binary can find libcef.so at runtime
    // without an LD_LIBRARY_PATH dance. `cef-dll-sys` extracts the CEF
    // binary distribution into `target/<profile>/build/cef-dll-sys-*/out/
    // cef_linux_x86_64/`. We probe in this order:
    //
    //   1. `DEP_CEF_DLL_CEF_DIR` (set by cef-dll-sys's `links = "cef"`
    //      metadata when cef-dll-sys is a direct dep).
    //   2. Glob `target/<profile>/build/cef-dll-sys-*/out/cef_linux_x86_64`
    //      as a fallback for the case where the metadata didn't propagate.
    //
    // Debug logging via `cargo:warning=` so failures are visible in
    // `cargo build` output.
    if let Some(cef_dir) = find_cef_dir() {
        println!("cargo:warning=CEF lib dir: {cef_dir}");
        println!("cargo:rustc-link-arg=-Wl,-rpath,{cef_dir}");
    } else {
        println!(
            "cargo:warning=CEF lib dir not found; binary will need LD_LIBRARY_PATH at runtime"
        );
    }

    tauri_build::build()
}

fn find_cef_dir() -> Option<String> {
    if let Ok(dir) = std::env::var("DEP_CEF_DLL_CEF_DIR") {
        return Some(dir);
    }

    // Walk up from OUT_DIR (...target/<profile>/build/<crate>-<hash>/out)
    // to the target dir, then glob the cef-dll-sys build outputs.
    let out_dir = std::env::var("OUT_DIR").ok()?;
    let target_dir = std::path::PathBuf::from(&out_dir)
        .ancestors()
        .nth(3)?
        .to_path_buf();
    let build_dir = target_dir.join("build");

    let entries = std::fs::read_dir(&build_dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("cef-dll-sys-") {
            continue;
        }
        let candidate = entry.path().join("out").join("cef_linux_x86_64");
        if candidate.join("libcef.so").is_file() {
            return Some(candidate.display().to_string());
        }
    }
    None
}
