import { onBeforeUnmount, onMounted } from 'vue';

/**
 * 监听页面恢复可见（解除锁屏 / 从睡眠唤醒 / 从后台切回），立即触发一次追赶刷新。
 *
 * 仅负责注册与卸载监听，刷新逻辑由调用方提供。解决系统睡眠 / 锁屏后
 * `setInterval` 被 WebView2 节流或挂起，导致前端状态长时间不更新的问题：
 * 唤醒后不必等到下一个 tick，而是立刻补一次刷新。
 *
 * @param onResume 页面重新可见时执行的回调（通常是立即触发一次状态刷新）。
 */
export function usePollingResume(onResume: () => void): void {
  const handler = () => {
    if (document.visibilityState === 'visible') {
      onResume();
    }
  };

  onMounted(() => {
    document.addEventListener('visibilitychange', handler);
    window.addEventListener('pageshow', handler);
  });

  onBeforeUnmount(() => {
    document.removeEventListener('visibilitychange', handler);
    window.removeEventListener('pageshow', handler);
  });
}
