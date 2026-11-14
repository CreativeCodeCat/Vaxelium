fn main() {
    // The page in ui/ is built into the app, so rebuild when it changes.
    println!("cargo:rerun-if-changed=ui");
    tauri_build::build()
}
