use crate::{
    hooks::{load_hooks, EnvConfigState},
    service::{ServiceImpl, ServiceState},
};
use anyhow::Context;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};
use tauri::Manager;
use tracing::error;

mod client;
mod common;
mod config;
mod hooks;
mod migration;
mod service;
pub mod test_support;
mod utils;

pub use anyhow::Result;
pub use config::{load_configs, save_configs, EnvConfig, StorageMode};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let service_closed = Arc::new(AtomicBool::new(false));
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 日志相关配置
            tauri::async_runtime::block_on(async { init(app).await })?;
            Ok(())
        });
    let builder = load_hooks(builder);
    let app = builder.build(tauri::generate_context!());
    let Ok(app) = app else {
        if let Err(err) = app {
            eprintln!("[startup] error while building tauri application: {err}");
        }
        return;
    };

    let shutdown_guard = Arc::clone(&service_closed);
    app.run(move |app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            if shutdown_guard.swap(true, Ordering::SeqCst) {
                return;
            }

            let service_state = app_handle.state::<ServiceState>();
            if let Err(err) =
                tauri::async_runtime::block_on(async { service_state.0.close().await })
            {
                error!(error = %err, "failed to close service during app shutdown");
            }
        }
    });
}

async fn init(app: &mut tauri::App) -> Result<()> {
    let config = config::load_configs()
        .map_err(|err| {
            eprintln!("[startup] failed to load configuration: {err}");
            err
        })
        .context("failed to load application configuration before logger initialization")?;

    app.manage(EnvConfigState(RwLock::new(config.clone())));

    config::init_log(&app.handle(), config.log_dir())
        .map_err(|err| {
            eprintln!("[startup] failed to initialize logging: {err}");
            err
        })
        .context("failed to initialize tracing logger")?;

    let service = service::init_service(config)
        .await
        .map_err(|err| {
            error!(error = %err, "failed to initialize service");
            err
        })
        .context("failed to initialize service backend")?;

    match &service {
        ServiceImpl::Local(local) => {
            tracing::info!(data_dir = %local.data_dir(), "local storage initialized");
        }
        ServiceImpl::Remote(_) => {
            tracing::info!("remote storage initialized");
        }
    }

    app.manage(ServiceState(service));

    Ok(())
}
