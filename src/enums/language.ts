export enum AppLanguage {
  Chinese = 'chinese',
  English = 'english',
  Japanese = 'japanese'
}

export const APP_LANGUAGE_LABELS: Record<AppLanguage, string> = {
  [AppLanguage.Chinese]: '中文（简体）',
  [AppLanguage.English]: 'English',
  [AppLanguage.Japanese]: '日本語'
};

export const APP_LANGUAGE_SHORT_LABELS: Record<AppLanguage, string> = {
  [AppLanguage.Chinese]: '中文',
  [AppLanguage.English]: '英文',
  [AppLanguage.Japanese]: '日文'
};
