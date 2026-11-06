import { invoke } from '@tauri-apps/api/core';

import type { HistoryRecord, HistoryRecordSummary } from '@/types/domain';

/**
 * 组装完整历史记录。
 *
 * `get_history_record` 自任务事件化（批次 10）起仅返回 summary 级
 * （detail/taskLog 字段置空），detail 与 taskLog 拆分至
 * `get_history_record_detail` / `get_history_task_log` 按需拉取；
 * 本函数并行请求后合并，供视图映射配置与详情弹窗使用。
 *
 * @param withLog 是否拉取任务日志（详情弹窗需要，列表映射不需要）。
 */
export const loadHistoryRecord = async (
  historyId: number,
  options: { withLog?: boolean } = {}
): Promise<HistoryRecord> => {
  const [base, detail, taskLog] = await Promise.all([
    invoke<HistoryRecordSummary>('get_history_record', { historyId }),
    invoke<unknown>('get_history_record_detail', { historyId }),
    options.withLog
      ? invoke<string | null>('get_history_task_log', { historyId, maxBytes: null })
      : Promise.resolve(null)
  ]);

  return { ...base, detail, taskLog } as HistoryRecord;
};
