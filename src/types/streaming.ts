import type { AppLanguage } from '@/enums/language';
import type { HardwareType } from '@/enums/settings';

/** 头像图片扩展名白名单（与后端 STREAMING_AVATAR_IMAGE_EXTENSIONS 对齐）。 */
export const IMAGE_FILE_EXTENSIONS = ['png', 'jpg', 'jpeg', 'webp', 'gif'] as const;

/**
 * 流式语音说话人来源类别。
 * voice-clone 使用名称、参考音频和参考文本；trained 从已有 Ready 说话人选择。
 * preset 为预留类型。
 */
export type StreamingSpeakerCategory = 'voice-clone' | 'preset' | 'trained';

/** 消息展示侧：同一说话人的消息固定一侧，缺省视为 right。 */
export type StreamingSpeakerSide = 'left' | 'right';

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
  /** 消息展示侧；缺省视为 right。 */
  side?: StreamingSpeakerSide;
  /** 头像原图绝对路径（创建任务时由后端复制进任务 sample 目录）。 */
  avatarPath?: string;
  /** 头像原始文件名（展示用）。 */
  avatarName?: string;
  createTime?: string;
  modifyTime?: string;
}

export type StreamingMessageStatus = 'streaming' | 'completed' | 'error';

/**
 * 流式语音聊天消息。每个说话人即用户本身，每次发送只产生一条消息，
 * contextId 即消息 id（后端以其命名 audio/<contextId>.wav）。
 * 任务创建时间由后端执行任务前生成、响应返回时填充，前端不赋值。
 */
export interface StreamingChatMessage {
  id: string;
  text: string;
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
