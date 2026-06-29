import { invoke } from '@tauri-apps/api/core';
import { ref, watch, type Ref } from 'vue';

import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { useUiStore } from '@/stores/ui';
import type { Page, PageRequest } from '@/types/domain';

interface UsePaginationOptions<TItem, TFilter> {
  /** Tauri 命令名 */
  command: string;
  /** 响应式筛选条件，变化时自动重置到第 1 页并重新查询 */
  filter: Ref<TFilter>;
  initialPageSize?: number;
  /** 对返回项做归一化（与 store 现有 normalize 一致） */
  normalize?: (item: TItem) => TItem;
  /** 筛选变化的防抖毫秒数，默认 300 */
  debounceMs?: number;
  /** 错误提示前缀 */
  errorLabel?: string;
}

/**
 * 统一封装服务端分页查询：维护 items/total/page/pageSize/loading，
 * 筛选条件变化时自动 debounce + 重置第 1 页 + 重新查询。
 * 所有请求经 PageRequest 封装与 Rust 层交互。
 */
export function usePagination<TItem, TFilter>(options: UsePaginationOptions<TItem, TFilter>) {
  const uiStore = useUiStore();
  const items = ref<TItem[]>([]) as Ref<TItem[]>;
  const total = ref(0);
  const page = ref(1);
  const pageSize = ref(options.initialPageSize ?? 10);
  const loading = ref(false);

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let requestSeed = 0;

  const totalPages = () => Math.max(1, Math.ceil(total.value / pageSize.value));

  const buildRequest = (): PageRequest<TFilter> => ({
    page: page.value,
    pageSize: pageSize.value,
    filter: options.filter.value,
  });

  const refresh = async () => {
    loading.value = true;
    const seed = ++requestSeed;
    try {
      const result = await invoke<Page<TItem>>(options.command, { request: buildRequest() });
      if (seed !== requestSeed) {
        return;
      }
      const rawItems = Array.isArray(result?.items) ? result.items : [];
      items.value = options.normalize ? rawItems.map(options.normalize) : rawItems;
      total.value = typeof result?.total === 'number' ? result.total : 0;
    } catch (error) {
      if (seed !== requestSeed) {
        return;
      }
      items.value = [];
      total.value = 0;
      uiStore.notifyError(formatErrorMessage(options.errorLabel ?? '加载列表失败', error));
    } finally {
      if (seed === requestSeed) {
        loading.value = false;
      }
    }
  };

  const setPage = (next: number) => {
    const target = Math.min(Math.max(1, next), totalPages());
    if (target === page.value) {
      return;
    }
    page.value = target;
    refresh();
  };

  const setPageSize = (next: number) => {
    if (next === pageSize.value) {
      return;
    }
    pageSize.value = next;
    page.value = 1;
    refresh();
  };

  const setFilter = (patch: Partial<TFilter>) => {
    options.filter.value = { ...options.filter.value, ...patch };
  };

  watch(
    options.filter,
    () => {
      page.value = 1;
      if (debounceTimer) {
        clearTimeout(debounceTimer);
      }
      debounceTimer = setTimeout(() => {
        refresh();
      }, options.debounceMs ?? 300);
    },
    { deep: true },
  );

  return {
    items,
    total,
    page,
    pageSize,
    loading,
    refresh,
    setPage,
    setPageSize,
    setFilter,
  };
}
