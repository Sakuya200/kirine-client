import { invoke } from '@tauri-apps/api/core';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

import { AppLanguage, APP_LANGUAGE_SHORT_LABELS } from '@/enums/language';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { SpeakerStatus } from '@/enums/status';
import { useUiStore } from '@/stores/ui';
import type { BaseModel, SpeakerFilter, SpeakerPagedResult, SpeakerProfile } from '@/types/domain';

interface CreateSpeakerPayload {
  name: string;
  languages: string[];
  samples: number;
  baseModel: BaseModel;
  description: string;
  status: SpeakerStatus;
  source: 'local';
}

interface UpdateSpeakerPayload {
  id: number;
  name: string;
  description: string;
}

interface ImportSpeakerPayload {
  baseModel: BaseModel;
  modelVersion: string;
  sourceModelDirPath: string;
  name: string;
  description: string;
  language: AppLanguage;
}

interface LoadSpeakersOptions {
  silent?: boolean;
  force?: boolean;
}

const normalizeSpeaker = (item: Partial<SpeakerProfile>): SpeakerProfile => {
  const languages = Array.isArray(item.languages) ? item.languages : [];
  const safeStatus: SpeakerStatus =
    item.status === SpeakerStatus.Ready || item.status === SpeakerStatus.Training || item.status === SpeakerStatus.Disabled
      ? item.status
      : SpeakerStatus.Disabled;

  return {
    id: typeof item.id === 'number' ? item.id : 0,
    name: item.name?.trim() || '',
    languages,
    samples: typeof item.samples === 'number' ? item.samples : 0,
    baseModel: typeof item.baseModel === 'string' ? item.baseModel.trim() : '',
    createTime: item.createTime ?? '',
    modifyTime: item.modifyTime ?? '',
    description: item.description?.trim() || '',
    status: safeStatus,
    source: item.source === 'local' || item.source === 'preset' || item.source === 'remote' ? item.source : 'remote'
  };
};

const normalizeSpeakers = (items: SpeakerProfile[]): SpeakerProfile[] => items.map(item => normalizeSpeaker(item));

export const useSpeakerStore = defineStore('speakers', () => {
  const speakers = ref<SpeakerProfile[]>([]);
  const isLoading = ref(false);
  const initialized = ref(false);
  const uiStore = useUiStore();
  let loadSpeakersPromise: Promise<void> | null = null;

  // 分页与筛选状态
  const page = ref(1);
  // 说话人卡片为 3 列网格，单页大小取 9 的倍数以填满整行
  const pageSize = ref(9);
  const total = ref(0);
  const totalPages = ref(1);
  const filter = ref<SpeakerFilter>({ keyword: null, status: null, language: null });

  // 统计（来自分页响应，不再依赖前端全量聚合）
  const stats = ref({ readyCount: 0, trainingCount: 0, disabledCount: 0, totalSamples: 0 });

  let requestSeed = 0;
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  const speakerCount = computed(() => total.value);
  const readyCount = computed(() => stats.value.readyCount);
  const trainingCount = computed(() => stats.value.trainingCount);
  const disabledCount = computed(() => stats.value.disabledCount);
  const totalSamples = computed(() => stats.value.totalSamples);

  const fetchPage = async (silent = false) => {
    isLoading.value = true;
    const seed = ++requestSeed;
    loadSpeakersPromise = (async () => {
      try {
        const result = await invoke<SpeakerPagedResult>('list_speaker_infos', {
          request: {
            page: page.value,
            pageSize: pageSize.value,
            filter: filter.value
          }
        });
        if (seed !== requestSeed) {
          return;
        }
        speakers.value = Array.isArray(result?.items) ? normalizeSpeakers(result.items) : [];
        total.value = typeof result?.total === 'number' ? result.total : 0;
        totalPages.value = typeof result?.totalPages === 'number' ? result.totalPages : 1;
        stats.value = {
          readyCount: result?.readyCount ?? 0,
          trainingCount: result?.trainingCount ?? 0,
          disabledCount: result?.disabledCount ?? 0,
          totalSamples: result?.totalSamples ?? 0
        };
      } catch (error) {
        if (seed !== requestSeed) {
          return;
        }
        speakers.value = [];
        total.value = 0;
        totalPages.value = 1;
        stats.value = { readyCount: 0, trainingCount: 0, disabledCount: 0, totalSamples: 0 };
        if (!silent) {
          uiStore.notifyError(formatErrorMessage('加载说话人列表失败', error));
        }
      } finally {
        if (seed === requestSeed) {
          isLoading.value = false;
          initialized.value = true;
          loadSpeakersPromise = null;
        }
      }
    })();

    return loadSpeakersPromise;
  };

  const loadSpeakers = async ({ silent = false }: LoadSpeakersOptions = {}) => {
    if (loadSpeakersPromise) {
      return loadSpeakersPromise;
    }
    return fetchPage(silent);
  };

  const refreshSpeakers = async (options: LoadSpeakersOptions = {}) => {
    await loadSpeakers(options);
  };

  const setPage = (next: number) => {
    const target = Math.min(Math.max(1, next), totalPages.value);
    if (target === page.value) {
      return;
    }
    page.value = target;
    fetchPage();
  };

  const setPageSize = (next: number) => {
    if (next === pageSize.value) {
      return;
    }
    pageSize.value = next;
    page.value = 1;
    fetchPage();
  };

  const setFilter = (patch: Partial<SpeakerFilter>) => {
    filter.value = { ...filter.value, ...patch };
    page.value = 1;
    if (debounceTimer) {
      clearTimeout(debounceTimer);
    }
    debounceTimer = setTimeout(() => {
      fetchPage();
    }, 300);
  };

  const ensureLoaded = async ({ force = false, silent = false }: LoadSpeakersOptions = {}) => {
    if (initialized.value && !force) {
      return;
    }

    await loadSpeakers({ silent });
  };

  const createSpeaker = async (payload: CreateSpeakerPayload) => {
    try {
      const created = normalizeSpeaker(
        await invoke<SpeakerProfile>('create_speaker_info', {
          payload: {
            name: payload.name,
            languages: payload.languages,
            samples: payload.samples,
            baseModel: payload.baseModel,
            description: payload.description,
            status: payload.status,
            source: payload.source
          }
        })
      );

      uiStore.notifySuccess(`已新增说话人“${created.name}”。`, 3200);
      await refreshSpeakers({ silent: true });
      return true;
    } catch (error) {
      uiStore.notifyError(formatErrorMessage('新增说话人失败', error));
      return false;
    }
  };

  const updateSpeaker = async (payload: UpdateSpeakerPayload) => {
    try {
      const updated = normalizeSpeaker(
        await invoke<SpeakerProfile>('update_speaker_info', {
          payload: {
            id: payload.id,
            name: payload.name,
            description: payload.description
          }
        })
      );

      uiStore.notifySuccess(`已更新说话人“${updated.name}”的信息。`, 3200);
      await refreshSpeakers({ silent: true });
      return true;
    } catch (error) {
      uiStore.notifyError(formatErrorMessage('保存说话人信息失败', error));
      return false;
    }
  };

  const importSpeaker = async (payload: ImportSpeakerPayload) => {
    try {
      const imported = normalizeSpeaker(
        await invoke<SpeakerProfile>('import_model_as_speaker', {
          payload: {
            baseModel: payload.baseModel,
            modelVersion: payload.modelVersion,
            sourceModelDirPath: payload.sourceModelDirPath,
            name: payload.name,
            description: payload.description,
            language: payload.language
          }
        })
      );

      speakers.value = [imported, ...speakers.value.filter(item => item.id !== imported.id)];
      uiStore.notifySuccess(`已导入说话人“${imported.name}”。`, 3200);
      await refreshSpeakers({ silent: true });
      return true;
    } catch (error) {
      uiStore.notifyError(formatErrorMessage('导入说话人失败', error));
      return false;
    }
  };

  const removeSpeaker = async (speakerId: number) => {
    const speaker = speakers.value.find(item => item.id === speakerId);
    const speakerName = speaker?.name ?? '';

    try {
      const deleted = await invoke<boolean>('delete_speaker_info', { speakerId });

      if (!deleted) {
        uiStore.notifyError('删除说话人失败。');
        return false;
      }

      uiStore.notifySuccess(`已删除说话人“${speakerName}”。`, 3200);
      await refreshSpeakers({ silent: true });
      return true;
    } catch (error) {
      uiStore.notifyError(formatErrorMessage('删除说话人失败', error));
      return false;
    }
  };

  const getLanguageLabel = (speaker: SpeakerProfile) =>
    speaker.languages.map(language => APP_LANGUAGE_SHORT_LABELS[language as AppLanguage] ?? language).join(' / ');

  return {
    speakers,
    isLoading,
    initialized,
    page,
    pageSize,
    total,
    totalPages,
    filter,
    speakerCount,
    readyCount,
    trainingCount,
    disabledCount,
    totalSamples,
    createSpeaker,
    ensureLoaded,
    loadSpeakers,
    refreshSpeakers,
    setPage,
    setPageSize,
    setFilter,
    updateSpeaker,
    importSpeaker,
    removeSpeaker,
    getLanguageLabel
  };
});
