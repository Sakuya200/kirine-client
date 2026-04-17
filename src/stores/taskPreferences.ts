import { defineStore } from 'pinia';
import { ref } from 'vue';

export const useTaskPreferencesStore = defineStore('taskPreferences', () => {
  const fixedBaseModel = ref('');

  return {
    fixedBaseModel
  };
});
