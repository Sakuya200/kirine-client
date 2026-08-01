import type { AppLanguage } from '@/enums/language';
import type { HardwareType } from '@/enums/settings';

/**
 * 流式语音说话人来源类别。
 * voice-clone 使用名称、参考音频和参考文本；trained 从已有 Ready 说话人选择。
 * preset 为预留类型。
 */
export type StreamingSpeakerCategory = 'voice-clone' | 'preset' | 'trained';

/**
 * 流式语音生成页说话人配置。voice-clone 由前端本地管理，
 * trained 关联已有说话人；时间字段由后端生成、前端只读。
 */
export interface StreamingSpeakerConfig {
  id: string;
  name: string;
  category: StreamingSpeakerCategory;
  /** trained 说话人 = speaker_id；voice-clone 无。 */
  speakerDirName?: string;
  baseModel: string;
  modelVersion?: string;
  refAudioPath: string;
  refAudioName: string;
  refText: string;
  description?: string;
  createTime?: string;
  modifyTime?: string;
}

export type StreamingMessageRole = 'user' | 'assistant';
export type StreamingMessageStatus = 'streaming' | 'completed' | 'error';

/**
 * 流式语音聊天消息。assistant 消息挂载 StreamableAudioPlayer(mode='stream')。
 * 任务创建时间由后端执行任务前生成、响应返回时填充，前端不赋值。
 */
export interface StreamingChatMessage {
  id: string;
  role: StreamingMessageRole;
  text: string;
  synthText: string; // 该轮待合成的文本（assistant 消息携带，传给 send_streaming_message）
  speakerId: string | null;
  speakerName?: string;
  taskId: number;
  contextId: string;
  audioPath?: string;
  status: StreamingMessageStatus;
  taskCreateTime?: string;
}

/**
 * 流式语音会话级配置（抽屉编辑、聊天页消费）。
 * 提交后端的 payload 不含时间字段，与 VoiceCloneView/TextToSpeechView 一致。
 */
export interface StreamingSessionConfig {
  baseModel: string;
  modelVersion: string;
  device: HardwareType;
  language: AppLanguage;
  modelParams: Record<string, unknown>;
}
