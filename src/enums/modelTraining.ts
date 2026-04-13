import { APP_LANGUAGE_SHORT_LABELS, AppLanguage } from './language';

export enum ModelTrainingSampleType {
  Single = 'single',
  Dataset = 'dataset'
}

export enum ModelTrainingAnnotationFormat {
  Jsonl = 'jsonl',
  Xlsx = 'xlsx',
  Xls = 'xls'
}

export interface ModelTrainingOption {
  label: string;
  value: AppLanguage;
}

export const MODEL_TRAINING_LANGUAGE_OPTIONS: ModelTrainingOption[] = [
  { value: AppLanguage.Chinese, label: APP_LANGUAGE_SHORT_LABELS[AppLanguage.Chinese] },
  { value: AppLanguage.English, label: APP_LANGUAGE_SHORT_LABELS[AppLanguage.English] },
  { value: AppLanguage.Japanese, label: APP_LANGUAGE_SHORT_LABELS[AppLanguage.Japanese] }
];

export const MODEL_TRAINING_SAMPLE_TYPE_TEXT: Record<ModelTrainingSampleType, string> = {
  [ModelTrainingSampleType.Single]: '单样本',
  [ModelTrainingSampleType.Dataset]: '样本集'
};

export const MODEL_TRAINING_ANNOTATION_FORMAT_TEXT: Record<ModelTrainingAnnotationFormat, string> = {
  [ModelTrainingAnnotationFormat.Jsonl]: 'JSONL',
  [ModelTrainingAnnotationFormat.Xlsx]: 'Excel (.xlsx)',
  [ModelTrainingAnnotationFormat.Xls]: 'Excel (.xls)'
};

export const MODEL_TRAINING_ANNOTATION_FILE_EXTENSIONS = [
  ModelTrainingAnnotationFormat.Jsonl,
  ModelTrainingAnnotationFormat.Xlsx,
  ModelTrainingAnnotationFormat.Xls
] as const;

export const MODEL_TRAINING_AUDIO_FILE_EXTENSIONS = ['wav', 'mp3', 'flac', 'ogg'] as const;
