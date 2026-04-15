import type { AppLanguage } from '@/enums/language';
import type { ModelTrainingSampleType } from '@/enums/modelTraining';
import type { BaseModel } from '@/enums/settings';
import type { SpeakerStatus, TaskStatus } from '@/enums/status';
import type { TextToSpeechFormat } from '@/enums/textToSpeech';
import type { HistoryTaskType } from '@/enums/task';

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
  modelName: string;
  epochCount: number;
  batchSize: number;
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
  language: AppLanguage;
  format: TextToSpeechFormat;
  text: string;
  voicePrompt: string;
  charCount: number;
  fileName: string;
  outputFilePath: string;
}

export interface VoiceCloneTaskDetail {
  baseModel: BaseModel;
  language: AppLanguage;
  format: TextToSpeechFormat;
  refAudioName: string;
  refAudioPath: string;
  refText: string;
  text: string;
  charCount: number;
  fileName: string;
  outputFilePath: string;
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
