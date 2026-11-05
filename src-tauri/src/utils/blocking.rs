use crate::Result;

/// 将同步重 IO 任务（大文件拷贝、zip 解压、模态对话框等待等）调度到专用阻塞
/// 线程池执行，避免占死 async runtime worker 导致其他命令卡顿。
///
/// 闭包必须自包含（`move` 进所需数据的 owned 副本），不得借用调用方状态；
/// `LocalService` 等引用计数克隆体可以直接 move 进闭包。
pub async fn run_blocking<T, F>(task: F) -> Result<T>
where
    F: FnOnce() -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|err| anyhow::anyhow!("blocking task join failed: {err}"))?
}
