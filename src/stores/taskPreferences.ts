import { defineStore } from 'pinia';
import { ref, watch } from 'vue';

import { BaseModel, HardwareType } from '@/enums/settings';

const TTS_HARDWARE_TYPE_KEY = 'kirine:tts-hardware-type';
const MODEL_TRAINING_HARDWARE_TYPE_KEY = 'kirine:model-training-hardware-type';
const VOICE_CLONE_HARDWARE_TYPE_KEY = 'kirine:voice-clone-hardware-type';

const isHardwareType = (value: string | null): value is HardwareType => value === HardwareType.Cuda || value === HardwareType.Cpu;

const loadStoredHardwareType = (storageKey: string) => {
  if (typeof window === 'undefined') {
    return HardwareType.Cuda;
  }

  const value = window.localStorage.getItem(storageKey);
  return isHardwareType(value) ? value : HardwareType.Cuda;
};

export const useTaskPreferencesStore = defineStore('taskPreferences', () => {
  const fixedBaseModel = ref<BaseModel>(BaseModel.Qwen3Tts);
  const ttsHardwareType = ref<HardwareType>(loadStoredHardwareType(TTS_HARDWARE_TYPE_KEY));
  const modelTrainingHardwareType = ref<HardwareType>(loadStoredHardwareType(MODEL_TRAINING_HARDWARE_TYPE_KEY));
  const voiceCloneHardwareType = ref<HardwareType>(loadStoredHardwareType(VOICE_CLONE_HARDWARE_TYPE_KEY));

  watch(ttsHardwareType, value => {
    window.localStorage.setItem(TTS_HARDWARE_TYPE_KEY, value);
  });

  watch(modelTrainingHardwareType, value => {
    window.localStorage.setItem(MODEL_TRAINING_HARDWARE_TYPE_KEY, value);
  });

  watch(voiceCloneHardwareType, value => {
    window.localStorage.setItem(VOICE_CLONE_HARDWARE_TYPE_KEY, value);
  });

  return {
    fixedBaseModel,
    ttsHardwareType,
    modelTrainingHardwareType,
    voiceCloneHardwareType
  };
});
