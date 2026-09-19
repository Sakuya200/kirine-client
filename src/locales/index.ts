import { createI18n } from 'vue-i18n';

import { DEFAULT_UI_LANGUAGE } from '@/enums/uiLanguage';
import enUS from './en-US';
import zhCN from './zh-CN';

export const i18n = createI18n({
  legacy: false,
  locale: DEFAULT_UI_LANGUAGE,
  fallbackLocale: DEFAULT_UI_LANGUAGE,
  messages: {
    'zh-CN': zhCN,
    'en-US': enUS
  },
  missingWarn: false,
  fallbackWarn: false
});
