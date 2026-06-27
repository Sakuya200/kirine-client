export enum AppLanguage {
  Chinese = 'chinese',
  English = 'english',
  Japanese = 'japanese',
  Korean = 'korean'
}

export const APP_LANGUAGE_LABELS: Record<AppLanguage, string> = {
  [AppLanguage.Chinese]: '中文（简体）',
  [AppLanguage.English]: 'English',
  [AppLanguage.Japanese]: '日本語',
  [AppLanguage.Korean]: '한국어'
};

export const APP_LANGUAGE_SHORT_LABELS: Record<AppLanguage, string> = {
  [AppLanguage.Chinese]: '中文',
  [AppLanguage.English]: '英文',
  [AppLanguage.Japanese]: '日文',
  [AppLanguage.Korean]: '韩文'
};
