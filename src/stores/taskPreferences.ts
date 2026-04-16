import { defineStore } from 'pinia';
import { ref } from 'vue';

import { BaseModel } from '@/enums/settings';

export const useTaskPreferencesStore = defineStore('taskPreferences', () => {
  const fixedBaseModel = ref<BaseModel>(BaseModel.Qwen3Tts);

  return {
    fixedBaseModel
  };
});
