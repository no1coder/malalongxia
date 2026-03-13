mod commands;

use commands::config::{
    check_app_update, check_feishu_plugin, check_openclaw_status, configure_api,
    configure_feishu, export_logs, install_feishu_plugin, launch_openclaw, open_url,
    openclaw_dashboard, openclaw_doctor, openclaw_health, repair_openclaw,
    reset_installation, restart_openclaw_gateway, stop_openclaw_gateway,
    test_api_connection, update_openclaw,
};
use commands::environment::check_environment;
use commands::config::uninstall_components;
use commands::install::{install_node, install_openclaw, verify_node_npm};
use commands::mirror::{fetch_mirror_config, test_mirror_latency, test_mirrors};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            check_environment,
            test_mirrors,
            test_mirror_latency,
            fetch_mirror_config,
            install_node,
            install_openclaw,
            verify_node_npm,
            configure_api,
            test_api_connection,
            launch_openclaw,
            export_logs,
            reset_installation,
            open_url,
            check_openclaw_status,
            update_openclaw,
            stop_openclaw_gateway,
            restart_openclaw_gateway,
            openclaw_doctor,
            openclaw_health,
            openclaw_dashboard,
            repair_openclaw,
            check_feishu_plugin,
            install_feishu_plugin,
            configure_feishu,
            check_app_update,
            uninstall_components,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
