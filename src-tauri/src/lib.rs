use crate::{
    hooks::{load_hooks, EnvConfigState, UiConfigState},
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
pub mod utils;

pub use anyhow::Result;
pub use config::{
    load_configs, load_ui_configs, load_ui_configs_from_dir, save_configs, ComponentProps,
    EnvConfig, ParamDefinition, SelectOption, StorageMode, TaskParamConfig, UiComponentType,
    UiConfigCatalog, UiParamType, VisibleWhenRule,
};
pub use service::models::HistoryTaskType;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // WebView2 Runtime 缺失时 builder.build 会失败并走 eprintln 静默退出，
    // 绿色包场景没有安装器的 downloadBootstrapper 兜底安装，用户视角即“闪退”。
    // 启动最前面先行检测，缺失时以原生弹窗明确提示。
    if tauri::webview_version().is_err() {
        show_error_dialog(
            "未检测到 Microsoft Edge WebView2 Runtime，Kirine Client 无法启动。\n\n\
             请从 https://developer.microsoft.com/microsoft-edge/webview2/ 安装 \
             Evergreen Runtime 后重试。",
        );
        return;
    }

    // 安装版以 NSIS 安装完成页“打开应用”等方式启动时，进程工作目录可能不是安装目录，
    // 而启动期的配置/src-model/本地服务路径均按相对路径解析，会导致找不到文件而闪退。
    // 发布构建下将工作目录锚定到可执行文件所在目录，保证相对路径解析与开发期一致。
    if !cfg!(debug_assertions) {
        if let Some(exe_dir) = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
        {
            let _ = std::env::set_current_dir(&exe_dir);
        }
    }

    let service_closed = Arc::new(AtomicBool::new(false));
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 应用后端初始化逻辑
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

#[cfg(windows)]
fn show_error_dialog(message: &str) {
    use std::{
        ffi::OsStr,
        iter::once,
        os::windows::ffi::OsStrExt,
    };

    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, MB_ICONERROR, MB_OK, MB_SETFOREGROUND,
    };

    fn to_wide(value: &str) -> Vec<u16> {
        OsStr::new(value).encode_wide().chain(once(0)).collect()
    }

    let text = to_wide(message);
    let caption = to_wide("Kirine Client");
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_OK | MB_ICONERROR | MB_SETFOREGROUND,
        );
    }
}

#[cfg(not(windows))]
fn show_error_dialog(message: &str) {
    eprintln!("{message}");
}

async fn init(app: &mut tauri::App) -> Result<()> {
    let config = config::load_configs()
        .map_err(|err| {
            eprintln!("[startup] failed to load configuration: {err}");
            err
        })
        .context("failed to load application configuration before logger initialization")?;
    let ui_config = config::load_ui_configs()
        .map_err(|err| {
            eprintln!("[startup] failed to load ui configuration: {err}");
            err
        })
        .context("failed to load ui configuration catalog")?;

    app.manage(EnvConfigState(RwLock::new(config.clone())));
    app.manage(UiConfigState(Arc::new(ui_config)));

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
