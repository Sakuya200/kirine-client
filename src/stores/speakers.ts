import { invoke } from '@tauri-apps/api/core';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

import { AppLanguage, APP_LANGUAGE_SHORT_LABELS } from '@/enums/language';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { SpeakerStatus } from '@/enums/status';
import { useUiStore } from '@/stores/ui';
import type { BaseModel, SpeakerProfile } from '@/types/domain';

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
  modelScale: string;
  sourceModelDirPath: string;
  name: string;
  description: string;
  language: AppLanguage;
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
    source: item.source === 'local' || item.source === 'remote' ? item.source : 'remote'
  };
};

const normalizeSpeakers = (items: SpeakerProfile[]): SpeakerProfile[] => items.map(item => normalizeSpeaker(item));

export const useSpeakerStore = defineStore('speakers', () => {
  const speakers = ref<SpeakerProfile[]>([]);
  const isLoading = ref(false);
  const initialized = ref(false);
  const uiStore = useUiStore();

  const speakerCount = computed(() => speakers.value.length);
  const readyCount = computed(() => speakers.value.filter(speaker => speaker.status === SpeakerStatus.Ready).length);
  const trainingCount = computed(() => speakers.value.filter(speaker => speaker.status === SpeakerStatus.Training).length);
  const disabledCount = computed(() => speakers.value.filter(speaker => speaker.status === SpeakerStatus.Disabled).length);
  const totalSamples = computed(() => speakers.value.reduce((total, speaker) => total + speaker.samples, 0));

  const loadSpeakers = async () => {
    isLoading.value = true;

    try {
      const result = await invoke<SpeakerProfile[]>('list_speaker_infos');
      speakers.value = Array.isArray(result) ? normalizeSpeakers(result) : [];
    } catch (error) {
      speakers.value = [];
      uiStore.notifyError(formatErrorMessage('加载说话人列表失败', error));
    } finally {
      isLoading.value = false;
      initialized.value = true;
    }
  };

  const refreshSpeakers = async () => {
    await loadSpeakers();
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

      speakers.value = [created, ...speakers.value];
      uiStore.notifySuccess(`已新增说话人“${created.name}”。`, 3200);
      return true;
    } catch (error) {
      uiStore.notifyError(formatErrorMessage('新增说话人失败', error));
      return false;
    }
  };

  const updateSpeaker = async (payload: UpdateSpeakerPayload) => {
    const speaker = speakers.value.find(item => item.id === payload.id);

    if (!speaker) {
      uiStore.notifyWarning('未找到目标说话人，无法保存修改。');
      return false;
    }

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

      speakers.value = speakers.value.map(item => (item.id === updated.id ? updated : item));
      uiStore.notifySuccess(`已更新说话人“${updated.name}”的信息。`, 3200);
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
            modelScale: payload.modelScale,
            sourceModelDirPath: payload.sourceModelDirPath,
            name: payload.name,
            description: payload.description,
            language: payload.language
          }
        })
      );

      speakers.value = [imported, ...speakers.value.filter(item => item.id !== imported.id)];
      uiStore.notifySuccess(`已导入说话人“${imported.name}”。`, 3200);
      return true;
    } catch (error) {
      uiStore.notifyError(formatErrorMessage('导入说话人失败', error));
      return false;
    }
  };

  const removeSpeaker = async (speakerId: number) => {
    const speaker = speakers.value.find(item => item.id === speakerId);

    if (!speaker) {
      uiStore.notifyWarning('未找到目标说话人，无法删除。');
      return false;
    }

    try {
      const deleted = await invoke<boolean>('delete_speaker_info', { speakerId });

      if (!deleted) {
        uiStore.notifyError('删除说话人失败。');
        return false;
      }

      speakers.value = speakers.value.filter(item => item.id !== speakerId);
      uiStore.notifySuccess(`已删除说话人“${speaker.name}”。`, 3200);
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
    speakerCount,
    readyCount,
    trainingCount,
    disabledCount,
    totalSamples,
    createSpeaker,
    loadSpeakers,
    refreshSpeakers,
    updateSpeaker,
    importSpeaker,
    removeSpeaker,
    getLanguageLabel
  };
});
