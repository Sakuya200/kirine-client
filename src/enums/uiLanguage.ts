export enum UiLanguage {
  Chinese = 'zh-CN',
  English = 'en-US'
}

export const DEFAULT_UI_LANGUAGE = UiLanguage.Chinese;

// 语言原生名称，语言无关，不走 i18n
export const UI_LANGUAGE_OPTIONS: { label: string; value: UiLanguage }[] = [
  { label: '中文（简体）', value: UiLanguage.Chinese },
  { label: 'English', value: UiLanguage.English }
];

export const isUiLanguage = (value: unknown): value is UiLanguage =>
  typeof value === 'string' && Object.values(UiLanguage).includes(value as UiLanguage);
