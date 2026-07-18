import type { AppLanguage } from '@/enums/language';

/**
 * 流式语音说话人来源类别。
 * 本期仅实现 voice-clone（名称 + 参考音频 + 参考文本），preset/trained 为预留，
 * 未来再确定是否支持（届时可接入现有 speakerStore 体系）。
 */
export type StreamingSpeakerCategory = 'voice-clone' | 'preset' | 'trained';

/**
 * 流式语音生成页本地说话人配置（语音克隆式）。
 * 不接入现有 speakerStore，纯前端本地管理；时间字段由后端生成、前端只读，
 * 纯前端本地阶段留空，未来接入后端持久化时由后端返回填充。
 */
export interface StreamingSpeakerConfig {
  id: string;
  name: string;
  category: StreamingSpeakerCategory;
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
  speakerId: string | null;
  speakerName?: string;
  taskId: number;
  contextId: string;
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
  device: string;
  language: AppLanguage;
  modelParams: Record<string, unknown>;
}
