import { invoke } from '@tauri-apps/api/core';

import { HistoryTaskType } from '@/enums/task';
import type { HistoryRecord, HistoryRecordSummary, Page } from '@/types/domain';

/**
 * 拉取指定任务类型最近的若干条历史记录（含 detail）。
 *
 * `list_history_records` 仅返回 `HistoryRecordSummary`（不含 detail），
 * 业务页（TTS/声音克隆/音色设计/模型微调）的结果卡需要完整记录用于映射配置，
 * 因此先按 taskType 取一页 summary，再通过 `get_history_record_detail`
 * 逐条并行拉取 detail 后合并（limit 很小，默认 5）。
 */
export const loadRecentHistoryRecords = async (
  taskType: HistoryTaskType,
  limit = 5
): Promise<HistoryRecord[]> => {
  const result = await invoke<Page<HistoryRecordSummary>>('list_history_records', {
    request: {
      page: 1,
      pageSize: limit,
      filter: { keyword: null, taskType, status: null }
    }
  });

  const summaries = result?.items ?? [];
  return Promise.all(
    summaries.map(async summary => {
      const detail = await invoke<unknown>('get_history_record_detail', {
        historyId: summary.id
      });
      return { ...summary, detail } as HistoryRecord;
    })
  );
};
