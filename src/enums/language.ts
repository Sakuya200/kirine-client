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

export const APP_LANGUAGE_SHORT_LABELS_KEY: Record<AppLanguage, string> = {
  [AppLanguage.Chinese]: 'common.languageName.chinese',
  [AppLanguage.English]: 'common.languageName.english',
  [AppLanguage.Japanese]: 'common.languageName.japanese',
  [AppLanguage.Korean]: 'common.languageName.korean'
};
