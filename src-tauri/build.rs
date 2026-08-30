fn main() {
    // Ensure that after replacing the icon, a `tauri dev` build is re-triggered (otherwise Cargo may not rerun build.rs and the Dock would still show the old icon).
    println!("cargo:rerun-if-changed=icons/icon.png");
    println!("cargo:rerun-if-changed=icons/icon.icns");
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    tauri_build::build()
}
