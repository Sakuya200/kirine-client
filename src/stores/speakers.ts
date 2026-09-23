import { invoke } from '@tauri-apps/api/core';
import { defineStore } from 'pinia';
import { i18n } from '@/locales';

import { SpeakerStatus } from '@/enums/status';
import { useUiStore } from '@/stores/ui';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import type { BaseModel, SpeakerProfile } from '@/types/domain';

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
 * 说话人写操作集合：仅封装 create / update / import / delete 四个 invoke 与结果通知。
 * 列表查询由各页面直连 `list_speaker_infos` 取数，本 store 不再缓存列表/分页/统计状态。
 */
export const useSpeakerStore = defineStore('speakers', () => {
  const uiStore = useUiStore();

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
    removeSpeaker
  };
});
