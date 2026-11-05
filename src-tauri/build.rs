fn main() {
    // Ensure that after replacing the icon, a `tauri dev` build is re-triggered (otherwise Cargo may not rerun build.rs and the Dock would still show the old icon).
    println!("cargo:rerun-if-changed=icons/icon.png");
    println!("cargo:rerun-if-changed=icons/icon.icns");
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=tauri.conf.json");

    let windows_msvc = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if !windows_msvc {
        tauri_build::build();
        return;
    }

    // Windows (MSVC): tauri-build embeds its app manifest — only a dependency
    // on Common Controls v6 — as a resource linked into bins alone, so the
    // `cargo test` harnesses load without it and die before any test runs
    // (STATUS_ENTRYPOINT_NOT_FOUND, TaskDialogIndirect; tauri-apps/tauri#13419).
    // Tauri's own workaround (examples/api/src-tauri/build.rs): skip the
    // resource manifest and have the linker embed the same file into every
    // artifact it links — the app and every test harness. Two manifests would
    // fail the link, hence `new_without_app_manifest`. `windows-app-manifest.xml`
    // is tauri-build 2's default, verbatim; `/MANIFESTUAC:NO` keeps the linker
    // from adding a UAC section the resource manifest never had.
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest()),
    )
    .expect("failed to run tauri-build");
    let manifest = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("windows-app-manifest.xml");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    println!("cargo:rustc-link-arg=/MANIFESTUAC:NO");
}
