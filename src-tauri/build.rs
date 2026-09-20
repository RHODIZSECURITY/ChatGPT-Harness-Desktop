fn main() {
    let manifest = tauri_build::AppManifest::new().commands(&["runtime_status"]);
    let attributes = tauri_build::Attributes::new().app_manifest(manifest);
    tauri_build::try_build(attributes).expect("failed to build RHODIZ Harness Desktop ACL");
}
