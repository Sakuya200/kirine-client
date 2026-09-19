import { invoke } from '@tauri-apps/api/core';
import { ref } from 'vue';

import { DEFAULT_UI_LANGUAGE, isUiLanguage, UiLanguage } from '@/enums/uiLanguage';
import { i18n } from '@/locales';
import { useUiStore } from '@/stores/ui';

const currentLanguage = ref<UiLanguage>(DEFAULT_UI_LANGUAGE);

interface LanguagePayload {
  language: string | null;
}

export const initUiLanguage = async () => {
  try {
    const payload = await invoke<LanguagePayload>('get_settings_config');
    const stored = payload?.language;

    if (isUiLanguage(stored)) {
      currentLanguage.value = stored;
      i18n.global.locale.value = stored;
    }
  } catch {
    // 读取失败保持默认中文，不打扰用户
  }
};

export const setUiLanguage = async (language: UiLanguage) => {
  const previous = currentLanguage.value;
  currentLanguage.value = language;
  i18n.global.locale.value = language;

  try {
    await invoke('save_ui_language', { language });
  } catch (error) {
    currentLanguage.value = previous;
    i18n.global.locale.value = previous;
    useUiStore().notifyError(String(error));
  }
};
