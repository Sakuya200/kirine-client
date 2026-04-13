use std::{fmt as stdfmt, fs, io, io::IsTerminal, path::PathBuf, sync::OnceLock, thread};

use anyhow::Context;
use tauri::{AppHandle, Runtime};
use time::{format_description::FormatItem, macros::format_description, OffsetDateTime, UtcOffset};
use tracing::Subscriber;
use tracing_appender::{non_blocking, non_blocking::WorkerGuard, rolling};
use tracing_subscriber::{
    fmt::{
        self,
        format::{FormatEvent, FormatFields, Writer},
        time::FormatTime,
        FmtContext,
    },
    layer::SubscriberExt,
    registry::LookupSpan,
    util::SubscriberInitExt,
    EnvFilter,
};

use crate::{config::resolve_storage_dir, Result};

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();
static LOCAL_TIME_OFFSET: OnceLock<UtcOffset> = OnceLock::new();

const LOG_FILE_PREFIX: &str = "kirine-client.log";
const CLIENT_TIME_FORMAT: &[FormatItem<'static>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]");

pub fn init_log<R: Runtime>(app: &AppHandle<R>, configured_log_dir: Option<&str>) -> Result<()> {
    let log_dir =
        resolve_log_dir(app, configured_log_dir).context("failed to resolve log directory")?;
    fs::create_dir_all(&log_dir)
        .with_context(|| format!("failed to create log directory: {}", log_dir.display()))?;

    let file_appender = rolling::daily(&log_dir, LOG_FILE_PREFIX);
    let (writer, guard) = non_blocking(file_appender);
    let _ = LOG_GUARD.set(guard);

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_directives()));

    let console_layer = fmt::layer()
        .with_ansi(io::stderr().is_terminal())
        .with_writer(io::stderr)
        .with_thread_names(false)
        .event_format(ClientLogFormatter::default());

    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(writer)
        .with_thread_names(false)
        .event_format(ClientLogFormatter::default());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .try_init()
        .context("failed to register tracing subscriber")?;

    tracing::info!(log_dir = %log_dir.display(), "logging initialized");
    Ok(())
}

pub fn resolve_base_log_dir(configured_log_dir: Option<&str>) -> Result<PathBuf> {
    let base_dir = resolve_storage_dir(configured_log_dir, "logs")?;
    fs::create_dir_all(&base_dir)
        .with_context(|| format!("failed to create log directory: {}", base_dir.display()))?;
    Ok(base_dir)
}

fn resolve_log_dir<R: Runtime>(
    _app: &AppHandle<R>,
    configured_log_dir: Option<&str>,
) -> Result<PathBuf> {
    resolve_base_log_dir(configured_log_dir)
}

fn default_directives() -> &'static str {
    if cfg!(debug_assertions) {
        "info,kirine_client_lib=debug,kirine_client=debug"
    } else {
        "info"
    }
}

#[derive(Default)]
struct ClientLogFormatter {
    timer: ClientLogTime,
}

impl<S, N> FormatEvent<S, N> for ClientLogFormatter
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> stdfmt::Result {
        self.timer.format_time(&mut writer)?;

        let metadata = event.metadata();
        let thread_name = current_thread_name();
        write!(
            writer,
            " {:<5} [{}] {} - ",
            metadata.level(),
            thread_name,
            metadata.target()
        )?;
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

#[derive(Default)]
struct ClientLogTime;

impl FormatTime for ClientLogTime {
    fn format_time(&self, writer: &mut Writer<'_>) -> stdfmt::Result {
        let now = now_with_local_offset();
        let formatted = now.format(CLIENT_TIME_FORMAT).map_err(|_| stdfmt::Error)?;
        writer.write_str(&formatted)
    }
}

fn now_with_local_offset() -> OffsetDateTime {
    OffsetDateTime::now_utc().to_offset(current_local_offset())
}

fn current_local_offset() -> UtcOffset {
    *LOCAL_TIME_OFFSET.get_or_init(|| UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC))
}

fn current_thread_name() -> String {
    match thread::current().name() {
        Some(name) if !name.is_empty() => name.to_owned(),
        _ => format!("thread-{:?}", thread::current().id()),
    }
}
