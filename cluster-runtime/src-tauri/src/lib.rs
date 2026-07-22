#![allow(dead_code)]

use std::path::PathBuf;

use tauri::Manager;

use cluster_runtime_core::bootstrap;
use cluster_runtime_core::AppState;

mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    cluster_runtime_core::logging::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("./data"));
            log::info!("app: data_dir={}", data_dir.display());
            let state = AppState::new(data_dir);
            app.manage(state);

            {
                let state = app.state::<AppState>();
                tauri::async_runtime::block_on(bootstrap::start(state.inner()));
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_system_info,
            commands::get_system_status,
            commands::get_activity,
            commands::get_nodes,
            commands::get_jobs,
            commands::get_plugins,
            commands::plugin_set_enabled,
            commands::plugin_install,
            commands::plugin_uninstall,
            commands::plugin_check_update,
            commands::get_metrics,
            commands::get_logs,
            commands::get_cluster_summary,
            commands::get_cluster_peers,
            commands::python_execute_code,
            commands::python_execute_script,
            commands::python_execute_module,
            commands::python_install_package,
            commands::python_uninstall_package,
            commands::python_list_packages,
            commands::python_create_environment,
            commands::python_delete_environment,
            commands::python_activate_environment,
            commands::python_runtime_health,
            commands::python_version,
            commands::python_list_environments,
            commands::python_package_index,
            commands::python_set_package_index,
            commands::dask_ensure_packages,
            commands::dask_get_settings,
            commands::dask_update_settings,
            commands::dask_start_scheduler,
            commands::dask_stop_scheduler,
            commands::dask_restart_scheduler,
            commands::dask_scheduler_status,
            commands::dask_start_worker,
            commands::dask_stop_worker,
            commands::dask_restart_worker,
            commands::dask_worker_status,
            commands::dask_connect_client,
            commands::dask_disconnect_client,
            commands::dask_cluster_snapshot,
            commands::dask_cluster_info,
            commands::dask_dashboard,
            commands::dask_metrics,
            commands::dask_submit_python_function,
            commands::dask_map,
            commands::dask_submit_script,
            commands::dask_submit_module,
            commands::dask_scatter,
            commands::dask_gather,
            commands::dask_job_status,
            commands::dask_run_example,
            commands::dask_cancel_job,
            commands::ray_ensure_packages,
            commands::ray_get_settings,
            commands::ray_update_settings,
            commands::ray_start_head,
            commands::ray_stop_head,
            commands::ray_restart_head,
            commands::ray_head_status,
            commands::ray_start_worker,
            commands::ray_stop_worker,
            commands::ray_restart_worker,
            commands::ray_worker_status,
            commands::ray_connect_client,
            commands::ray_disconnect_client,
            commands::ray_cluster_snapshot,
            commands::ray_cluster_info,
            commands::ray_dashboard,
            commands::ray_metrics,
            commands::ray_submit_python_function,
            commands::ray_map,
            commands::ray_submit_script,
            commands::ray_submit_module,
            commands::ray_scatter,
            commands::ray_gather,
            commands::ray_job_status,
            commands::ray_run_example,
            commands::ray_cancel_job,
            commands::job_submit,
            commands::job_cancel,
            commands::job_status,
            commands::job_progress,
            commands::job_result,
            commands::job_list,
            commands::job_get,
            commands::job_retry,
            commands::scheduler_list,
            commands::scheduler_get_active,
            commands::scheduler_set_active,
            commands::mpi_ensure_toolchain,
            commands::mpi_get_settings,
            commands::mpi_update_settings,
            commands::mpi_status,
            commands::p2p_local_peer_id,
            commands::p2p_listen_addrs,
            commands::p2p_connect,
            commands::check_for_updates,
            commands::get_app_version,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                let state = app_handle.state::<AppState>();
                tauri::async_runtime::block_on(bootstrap::shutdown_services(state.inner()));
            }
        });
}
