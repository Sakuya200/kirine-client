import { listen } from '@tauri-apps/api/event';
import { onScopeDispose } from 'vue';

import type { HistoryTaskType } from '@/enums/task';
import type { TaskStatus } from '@/enums/status';

/**
 * 任务列表失效事件订阅（对应后端 `history:task-updated`）。
 *
 * 模块级单例：整个应用只注册一次 Tauri listen，多订阅者本地扇出；
 * 500ms 防抖把状态突变期间的连续事件（created→updated→...）合并为
 * 一次回调，视图据此刷新最近任务与活动任务状态，替代 3s 轮询。
 */

export const HISTORY_TASK_UPDATED_EVENT = 'history:task-updated';

export type HistoryTaskEventAction = 'created' | 'updated' | 'deleted' | 'cancelled';

export interface HistoryTaskEventPayload {
  historyId: number;
  taskType: HistoryTaskType;
  status: TaskStatus;
  modifyTime: string;
  action: HistoryTaskEventAction;
}

export type HistoryTaskEventListener = (event: HistoryTaskEventPayload) => void;

const EVENT_DEBOUNCE_MS = 500;

let sharedUnlisten: Promise<() => void> | null = null;
const listeners = new Set<HistoryTaskEventListener>();
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
let pendingEvent: HistoryTaskEventPayload | null = null;

const ensureSharedListener = () => {
  if (!sharedUnlisten) {
    sharedUnlisten = listen<HistoryTaskEventPayload>(HISTORY_TASK_UPDATED_EVENT, event => {
      pendingEvent = event.payload;
      if (debounceTimer !== null) {
        return;
      }
      debounceTimer = setTimeout(() => {
        debounceTimer = null;
        const latest = pendingEvent;
        pendingEvent = null;
        if (latest) {
          listeners.forEach(listener => listener(latest));
        }
      }, EVENT_DEBOUNCE_MS);
    });
  }
  return sharedUnlisten;
};

/**
 * 订阅任务状态失效事件。回调收到的 payload 仅作失效信号，
 * 刷新逻辑（最近任务列表 / 活动任务状态）由调用方决定。
 */
export function useHistoryTaskEvents(listener: HistoryTaskEventListener): void {
  void ensureSharedListener();
  listeners.add(listener);

  onScopeDispose(() => {
    listeners.delete(listener);
  });
}
