import type { AppLanguage } from '@/enums/language';
import type { ModelTrainingSampleType } from '@/enums/modelTraining';
import type { HardwareType } from '@/enums/settings';
import type { SpeakerStatus, TaskStatus } from '@/enums/status';
import type { TextToSpeechFormat } from '@/enums/textToSpeech';
import type { HistoryTaskType } from '@/enums/task';

export type BaseModel = string;

export type SpeakerSource = 'local' | 'preset' | 'remote';

export interface SpeakerProfile {
  id: number;
  speakerName: string;
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
  device: HardwareType;
  createTime: string;
  modifyTime: string;
  taskLog?: string | null;
}

export interface ModelTrainingTaskDetail {
  language: AppLanguage;
  baseModel: BaseModel;
  modelVersion: string;
  speakerName: string;
  description: string;
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
  speakerId: number | null;
  baseModel: BaseModel;
  modelVersion: string;
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
  modelVersion: string;
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

export interface VoiceDesignTaskDetail {
  baseModel: BaseModel;
  modelVersion: string;
  language: AppLanguage;
  format: TextToSpeechFormat;
  exportAudioName: string;
  prompt: string;
  text: string;
  modelParams: Record<string, unknown>;
  charCount: number;
  fileName: string;
  outputFilePath: string;
}

export interface StreamingTaskDetail {
  baseModel: BaseModel;
  modelVersion: string;
  language: AppLanguage;
  device: HardwareType;
  modelParams: Record<string, unknown>;
  contextFilePath: string;
  inputCacheFilePath: string;
  outputAudioDir: string;
  messageCount: number;
}

export interface StreamingReplayMessage {
  historyId: number;
  messageId: string;
  contextId: string;
  speakerName: string;
  text: string;
  audioPath: string;
}

export interface StreamingReplaySnapshot {
  taskId: number;
  baseModel: BaseModel;
  modelVersion: string;
  language: AppLanguage;
  device: HardwareType;
  modelParams: Record<string, unknown>;
  speakers: StreamingSpeakerPayload[];
  messages: StreamingReplayMessage[];
}

export interface StreamingSpeakerPayload {
  name: string;
  baseModel: BaseModel;
  modelVersion?: string;
  refAudioPath: string;
  refAudioName: string;
  refText: string;
  description?: string;
  category?: 'voice-clone' | 'preset' | 'trained';
  speakerDirName?: string;
  /** 消息展示侧；缺省视为 right。 */
  side?: 'left' | 'right';
  /** 头像原图绝对路径（创建任务时由后端复制进任务 sample 目录）。 */
  avatarPath?: string;
  /** 头像原始文件名（展示用）。 */
  avatarName?: string;
}

export interface CreateStreamingSpeechTaskPayload {
  baseModel: BaseModel;
  modelVersion: string;
  device: HardwareType;
  language: AppLanguage;
  modelParams: Record<string, unknown>;
  speakers: StreamingSpeakerPayload[];
}

export interface SendStreamingMessagePayload {
  taskId: number;
  contextId: string;
  speakerName: string;
  text: string;
}

export interface StreamingSpeechTaskResult {
  taskId: number;
  contextFilePath: string;
  inputCacheFilePath: string;
  outputAudioDir: string;
  status: TaskStatus;
  createdAt: string;
}

export interface ModelInfo {
  id: number;
  baseModel: BaseModel;
  modelName: string;
  modelVersion: string;
  requiredModelNameList: string[];
  requiredModelRepoIdList: string[];
  supportedFeatureList: string[];
  supportedDevices: HardwareType[];
  currentDevice: HardwareType | null;
  supportedLanguages: AppLanguage[];
  downloaded: boolean;
  createTime: string;
  modifyTime: string;
}

export interface ModelMutationResult {
  model: ModelInfo;
  removedPaths: string[];
  preservedPaths: string[];
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

export interface VoiceDesignHistoryRecord extends HistoryRecordBase {
  taskType: HistoryTaskType.VoiceDesign;
  detail: VoiceDesignTaskDetail;
}

export interface StreamingSpeechHistoryRecord extends HistoryRecordBase {
  taskType: HistoryTaskType.StreamingSpeech;
  detail: StreamingTaskDetail;
}

export type HistoryRecord =
  | ModelTrainingHistoryRecord
  | TextToSpeechHistoryRecord
  | VoiceCloneHistoryRecord
  | VoiceDesignHistoryRecord
  | StreamingSpeechHistoryRecord;

/** 历史任务列表摘要（不含 detail/taskLog，用于分页列表查询） */
export type HistoryRecordSummary = HistoryRecordBase;

/** 统一分页请求结构（与 Rust 层 PageRequest<T> 对齐） */
export interface PageRequest<TFilter> {
  page: number;
  pageSize: number;
  filter?: TFilter | null;
}

/** 通用分页响应结构 */
export interface Page<T> {
  items: T[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

export interface SpeakerFilter {
  keyword?: string | null;
  status?: SpeakerStatus | null;
}

export interface ModelFilter {
  keyword?: string | null;
  downloaded?: boolean | null;
  feature?: HistoryTaskType | null;
}

export interface HistoryFilter {
  keyword?: string | null;
  taskType?: HistoryTaskType | null;
  status?: TaskStatus | null;
}

/** 说话人分页响应（附带统计，供页面统计卡使用） */
export interface SpeakerPagedResult {
  items: SpeakerProfile[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
  readyCount: number;
  trainingCount: number;
  disabledCount: number;
  totalSamples: number;
}
