import type { AppLanguage } from '@/enums/language';
import type { ModelTrainingSampleType } from '@/enums/modelTraining';
import type { SpeakerStatus, TaskStatus } from '@/enums/status';
import type { TextToSpeechFormat } from '@/enums/textToSpeech';
import type { HistoryTaskType } from '@/enums/task';

export type BaseModel = string;

export type SpeakerSource = 'local' | 'remote';

export interface SpeakerProfile {
  id: number;
  name: string;
  languages: string[];
  samples: number;
  baseModel: BaseModel;
  createTime: string;
  modifyTime: string;
  description: string;
  status: SpeakerStatus;
  source: SpeakerSource;
}

export interface HistoryRecordBase {
  id: number;
  taskType: HistoryTaskType;
  title: string;
  speaker: string;
  status: TaskStatus;
  durationSeconds: number;
  createTime: string;
  modifyTime: string;
  errorMessage?: string | null;
}

export interface ModelTrainingTaskDetail {
  language: AppLanguage;
  baseModel: BaseModel;
  modelScale: string;
  modelName: string;
  modelParams: Record<string, unknown>;
  sampleCount: number;
  samples: ModelTrainingSampleDetail[];
  notes: string[];
}

export interface ModelTrainingFileDetail {
  fileName: string;
  fileKind: 'audio' | 'archive' | 'annotation';
  filePath: string;
}

export interface ModelTrainingSampleDetail {
  id: number;
  sampleType: ModelTrainingSampleType;
  title: string;
  detail: string;
  transcriptPreview?: string | null;
  primaryFile: ModelTrainingFileDetail;
  secondaryFile?: ModelTrainingFileDetail | null;
}

export interface TextToSpeechTaskDetail {
  speakerId: number;
  baseModel: BaseModel;
  modelScale: string;
  language: AppLanguage;
  format: TextToSpeechFormat;
  exportAudioName: string;
  text: string;
  modelParams: Record<string, unknown>;
  charCount: number;
  fileName: string;
  outputFilePath: string;
}

export interface VoiceCloneTaskDetail {
  baseModel: BaseModel;
  modelScale: string;
  language: AppLanguage;
  format: TextToSpeechFormat;
  exportAudioName: string;
  refAudioName: string;
  refAudioPath: string;
  refText: string;
  text: string;
  modelParams: Record<string, unknown>;
  charCount: number;
  fileName: string;
  outputFilePath: string;
}

export interface ModelInfo {
  id: number;
  baseModel: BaseModel;
  modelName: string;
  modelScale: string;
  requiredModelNameList: string[];
  requiredModelRepoIdList: string[];
  supportedFeatureList: HistoryTaskType[];
  createTime: string;
  modifyTime: string;
}

export interface ModelTrainingHistoryRecord extends HistoryRecordBase {
  taskType: HistoryTaskType.ModelTraining;
  detail: ModelTrainingTaskDetail;
}

export interface TextToSpeechHistoryRecord extends HistoryRecordBase {
  taskType: HistoryTaskType.TextToSpeech;
  detail: TextToSpeechTaskDetail;
}

export interface VoiceCloneHistoryRecord extends HistoryRecordBase {
  taskType: HistoryTaskType.VoiceClone;
  detail: VoiceCloneTaskDetail;
}

export type HistoryRecord = ModelTrainingHistoryRecord | TextToSpeechHistoryRecord | VoiceCloneHistoryRecord;
