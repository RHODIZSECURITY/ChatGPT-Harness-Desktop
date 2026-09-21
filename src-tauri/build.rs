fn main() {
    let manifest = tauri_build::AppManifest::new().commands(&[
        "runtime_status",
        "runtime_provision",
        "runtime_start",
        "runtime_stop",
        "runtime_verify",
        "runtime_repair",
        "runtime_logs",
    ]);
    let attributes = tauri_build::Attributes::new().app_manifest(manifest);
    tauri_build::try_build(attributes).expect("failed to build RHODIZ Harness Desktop ACL");
}
