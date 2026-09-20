mod broker;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![broker::runtime_status])
        .run(tauri::generate_context!())
        .expect("error while running RHODIZ Harness Desktop");
}
