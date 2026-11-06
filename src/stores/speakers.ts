import { invoke } from '@tauri-apps/api/core';
import { defineStore } from 'pinia';
import { ref } from 'vue';
import { i18n } from '@/locales';

import { SpeakerStatus } from '@/enums/status';
import { useUiStore } from '@/stores/ui';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import type { BaseModel, SpeakerPagedResult, SpeakerProfile } from '@/types/domain';

interface CreateSpeakerPayload {
  speakerName: string;
  samples: number;
  baseModel: BaseModel;
  description: string;
  status: SpeakerStatus;
  source: 'local';
}

interface UpdateSpeakerPayload {
  id: number;
  speakerName: string;
  description: string;
  /** Some = 用该路径文件覆盖头像；undefined 且 removeAvatar=false = 保持不变 */
  avatarSourcePath?: string | null;
  /** true = 清除头像，优先于 avatarSourcePath */
  removeAvatar?: boolean;
}

interface ImportSpeakerPayload {
  baseModel: BaseModel;
  modelVersion: string;
  sourceModelDirPath: string;
  speakerName: string;
  description: string;
  /** 可选：导入时一并设置的头像文件路径 */
  avatarSourcePath?: string | null;
}

export const normalizeSpeaker = (item: Partial<SpeakerProfile>): SpeakerProfile => {
  const safeStatus: SpeakerStatus =
    item.status === SpeakerStatus.Ready || item.status === SpeakerStatus.Training || item.status === SpeakerStatus.Disabled
      ? item.status
      : SpeakerStatus.Disabled;

  return {
    id: typeof item.id === 'number' ? item.id : 0,
    speakerName: item.speakerName?.trim() || '',
    samples: typeof item.samples === 'number' ? item.samples : 0,
    baseModel: typeof item.baseModel === 'string' ? item.baseModel.trim() : '',
    createTime: item.createTime ?? '',
    modifyTime: item.modifyTime ?? '',
    description: item.description?.trim() || '',
    status: safeStatus,
    source: item.source === 'local' || item.source === 'preset' || item.source === 'remote' ? item.source : 'remote',
    avatarContentType: typeof item.avatarContentType === 'string' && item.avatarContentType.length > 0 ? item.avatarContentType : null
  };
};

/**
 * 说话人集合：写操作封装 + Ready 列表跨页缓存。
 * TTS 表单与流式说话人表单都按 status=Ready(+baseModel) 拉下拉数据，改为共享一份
 * 全量列表缓存（TTL + 在途去重），调用方按需客户端过滤；SpeakersView 的服务端
 * 分页/关键词检索/统计仍直连 `list_speaker_infos`，不走此缓存。
 */
export const useSpeakerStore = defineStore('speakers', () => {
  const uiStore = useUiStore();

  // —— Ready 说话人列表跨页缓存 ——
  // 写操作（创建/更新/导入/删除）成功后失效；TTL 内的跨页切换/表单重开共享同一份数据。
  const SPEAKER_CACHE_TTL_MS = 30_000;
  const cachedSpeakers = ref<SpeakerProfile[] | null>(null);
  const cachedSpeakersAt = ref(0);
  let speakersRequest: Promise<SpeakerProfile[]> | null = null;

  const invalidateSpeakerCache = () => {
    cachedSpeakers.value = null;
    cachedSpeakersAt.value = 0;
  };

  const fetchAllSpeakers = async (): Promise<SpeakerProfile[]> => {
    if (!speakersRequest) {
      speakersRequest = invoke<SpeakerPagedResult>('list_speaker_infos', {
        request: { page: 1, pageSize: 1000, filter: { keyword: null, status: null, baseModel: null } }
      })
        .then(result => (Array.isArray(result?.items) ? result.items.map(normalizeSpeaker) : []))
        .finally(() => {
          speakersRequest = null;
        });
    }
    return speakersRequest;
  };

  /** status=Ready 的说话人（可按 baseModel 过滤），供任务表单的下拉选项使用。 */
  const listReadySpeakers = async (
    { baseModel = null, force = false }: { baseModel?: BaseModel | null; force?: boolean } = {}
  ): Promise<SpeakerProfile[]> => {
    let items = cachedSpeakers.value;
    if (force || !items || Date.now() - cachedSpeakersAt.value >= SPEAKER_CACHE_TTL_MS) {
      items = await fetchAllSpeakers();
      cachedSpeakers.value = items;
      cachedSpeakersAt.value = Date.now();
    }
    return items.filter(
      speaker => speaker.status === SpeakerStatus.Ready && (!baseModel || speaker.baseModel === baseModel)
    );
  };

  const createSpeaker = async (payload: CreateSpeakerPayload) => {
    try {
      const created = normalizeSpeaker(
        await invoke<SpeakerProfile>('create_speaker_info', {
          payload: {
            speakerName: payload.speakerName,
            samples: payload.samples,
            baseModel: payload.baseModel,
            description: payload.description,
            status: payload.status,
            source: payload.source
          }
        })
      );

      uiStore.notifySuccess(i18n.global.t('common.store.speakers.created', { name: created.speakerName }), 3200);
      invalidateSpeakerCache();
      return true;
    } catch (error) {
      uiStore.notifyError(formatErrorMessage(i18n.global.t('common.store.speakers.createFailed'), error));
      return false;
    }
  };

  const updateSpeaker = async (payload: UpdateSpeakerPayload) => {
    try {
      const updated = normalizeSpeaker(
        await invoke<SpeakerProfile>('update_speaker_info', {
          payload: {
            id: payload.id,
            speakerName: payload.speakerName,
            description: payload.description,
            avatarSourcePath: payload.avatarSourcePath ?? null,
            removeAvatar: payload.removeAvatar ?? false
          }
        })
      );

      uiStore.notifySuccess(i18n.global.t('common.store.speakers.updated', { name: updated.speakerName }), 3200);
      invalidateSpeakerCache();
      return true;
    } catch (error) {
      uiStore.notifyError(formatErrorMessage(i18n.global.t('common.store.speakers.updateFailed'), error));
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
            speakerName: payload.speakerName,
            description: payload.description,
            avatarSourcePath: payload.avatarSourcePath ?? null
          }
        })
      );

      uiStore.notifySuccess(i18n.global.t('common.store.speakers.imported', { name: imported.speakerName }), 3200);
      invalidateSpeakerCache();
      return true;
    } catch (error) {
      uiStore.notifyError(formatErrorMessage(i18n.global.t('common.store.speakers.importFailed'), error));
      return false;
    }
  };

  const removeSpeaker = async (speakerId: number, speakerName = '') => {
    try {
      const deleted = await invoke<boolean>('delete_speaker_info', { speakerId });

      if (!deleted) {
        uiStore.notifyError(i18n.global.t('common.store.speakers.deleteFailed'));
        return false;
      }

      uiStore.notifySuccess(i18n.global.t('common.store.speakers.deleted', { name: speakerName }), 3200);
      invalidateSpeakerCache();
      return true;
    } catch (error) {
      uiStore.notifyError(formatErrorMessage(i18n.global.t('common.store.speakers.deleteFailed'), error));
      return false;
    }
  };

  return {
    createSpeaker,
    updateSpeaker,
    importSpeaker,
    removeSpeaker,
    listReadySpeakers,
    invalidateSpeakerCache
  };
});
