mod broker;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            broker::runtime_status,
            broker::runtime_provision,
            broker::runtime_start,
            broker::runtime_stop,
            broker::runtime_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running RHODIZ Harness Desktop");
}
