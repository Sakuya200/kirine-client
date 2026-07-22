import { defineStore } from 'pinia';
import { computed, reactive, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

import { AppLanguage } from '@/enums/language';
import { HardwareType } from '@/enums/settings';
import type { StreamingSpeechTaskResult } from '@/types/domain';
import type { StreamingChatMessage, StreamingSessionConfig, StreamingSpeakerCategory, StreamingSpeakerConfig } from '@/types/streaming';

/**
 * 需要参考文本的模型集合（对齐 TextToSpeechView 的 DYNAMIC_REFERENCE_BASE_MODELS）。
 * 后续可由后端模型能力字段驱动，本期以前端常量判断。
 */
export const REQUIRES_REF_TEXT_MODELS = new Set(['gpt_sovits_cpufast']);
export const requiresRefText = (baseModel: string) => REQUIRES_REF_TEXT_MODELS.has(baseModel);

/** 新增/编辑说话人时由表单提交的输入（不含 id/时间，时间由后端生成）。 */
export interface StreamingSpeakerInput {
  name: string;
  baseModel: string;
  modelVersion?: string;
  refAudioPath: string;
  refAudioName: string;
  refText: string;
  description?: string;
  /** 说话人来源类别，缺省视为 voice-clone。trained 时从 speakerStore 选择已训练说话人。 */
  category?: StreamingSpeakerCategory;
  /** trained 说话人 = speaker_id；voice-clone 无。 */
  speakerDirName?: string;
}

let speakerSeed = 0;
let messageSeed = 0;
const nextSpeakerId = () => `spk-${++speakerSeed}`;
const nextMessageId = () => `msg-${++messageSeed}`;

export const useStreamingSpeechStore = defineStore('streaming-speech', () => {
  const speakers = ref<StreamingSpeakerConfig[]>([]);
  const messages = ref<StreamingChatMessage[]>([]);
  const isDrawerOpen = ref(false);
  const activeTaskId = ref<number | null>(null);
  const isStartingSession = ref(false);
  const sessionConfig = reactive<StreamingSessionConfig>({
    baseModel: '',
    modelVersion: '',
    device: HardwareType.Cpu,
    language: AppLanguage.Chinese,
    modelParams: {}
  });

  const speakerOptions = computed(() =>
    speakers.value.map(speaker => ({
      label: speaker.name,
      value: speaker.id,
      description: speaker.description || `参考音频：${speaker.refAudioName || '未设置'}`
    }))
  );

  const getSpeaker = (id: string | null) => speakers.value.find(item => item.id === id) ?? null;

  const addSpeaker = (payload: StreamingSpeakerInput): StreamingSpeakerConfig => {
    const speaker: StreamingSpeakerConfig = {
      id: nextSpeakerId(),
      category: payload.category ?? 'voice-clone',
      speakerDirName: payload.speakerDirName,
      name: payload.name,
      baseModel: payload.baseModel,
      modelVersion: payload.modelVersion,
      refAudioPath: payload.refAudioPath,
      refAudioName: payload.refAudioName,
      refText: payload.refText,
      description: payload.description
    };
    speakers.value = [...speakers.value, speaker];
    return speaker;
  };

  const updateSpeaker = (id: string, patch: Partial<StreamingSpeakerInput>) => {
    speakers.value = speakers.value.map(item =>
      item.id === id
        ? {
            ...item,
            ...patch,
            id: item.id,
            category: item.category
          }
        : item
    );
  };

  const removeSpeaker = (id: string) => {
    speakers.value = speakers.value.filter(item => item.id !== id);
  };

  const openDrawer = () => {
    isDrawerOpen.value = true;
  };
  const closeDrawer = () => {
    isDrawerOpen.value = false;
  };
  const toggleDrawer = () => {
    isDrawerOpen.value = !isDrawerOpen.value;
  };

  const setSessionConfig = (patch: Partial<StreamingSessionConfig>) => {
    Object.assign(sessionConfig, patch);
  };

  /**
   * 发送一条聊天消息：首条消息时 invoke create_streaming_speech_task 拿 taskId 回填，
   * 随后 push user 消息 + 占位 assistant 消息。
   * 实际流式接收由渲染出的 StreamableAudioPlayer(mode='stream') 在 watch immediate 时
   * 自动调 startStreaming(taskId, contextId, speakerName, synthText) 完成。
   * 提交后端的 payload 不含时间字段，任务创建时间由后端执行前生成。
   */
  const sendMessage = async (text: string, speakerId: string | null) => {
    const trimmed = text.trim();
    if (trimmed.length === 0) {
      return;
    }
    // 防重入：首条消息建会话期间（isStartingSession=true）忽略后续发送，
    // 避免连点/回车连击触发多次 create_streaming_speech_task 产生僵尸会话。
    if (isStartingSession.value) {
      return;
    }
    const speaker = getSpeaker(speakerId);

    // 首条消息：建会话拿 taskId
    if (activeTaskId.value === null) {
      if (speakers.value.length === 0) {
        return;
      }
      isStartingSession.value = true;
      try {
        const result = await invoke<StreamingSpeechTaskResult>('create_streaming_speech_task', {
          payload: {
            baseModel: sessionConfig.baseModel,
            modelVersion: sessionConfig.modelVersion,
            device: sessionConfig.device,
            language: sessionConfig.language,
            modelParams: sessionConfig.modelParams,
            speakers: speakers.value.map(s => ({
              name: s.name,
              baseModel: s.baseModel,
              modelVersion: s.modelVersion,
              refAudioPath: s.refAudioPath,
              refAudioName: s.refAudioName,
              refText: s.refText,
              description: s.description,
              category: s.category,
              speakerDirName: s.speakerDirName
            }))
          }
        });
        activeTaskId.value = result.taskId;
      } finally {
        isStartingSession.value = false;
      }
    }

    const taskId = activeTaskId.value;
    if (taskId === null) {
      return;
    }

    const userMessage: StreamingChatMessage = {
      id: nextMessageId(),
      role: 'user',
      text: trimmed,
      synthText: trimmed,
      speakerId,
      speakerName: speaker?.name,
      taskId,
      contextId: '',
      status: 'completed'
    };
    messages.value = [...messages.value, userMessage];

    const assistantId = nextMessageId();
    const assistantMessage: StreamingChatMessage = {
      id: assistantId,
      role: 'assistant',
      text: '',
      synthText: trimmed,
      speakerId,
      speakerName: speaker?.name,
      taskId,
      contextId: assistantId,
      status: 'streaming'
    };
    messages.value = [...messages.value, assistantMessage];
  };

  const terminateSession = async () => {
    const taskId = activeTaskId.value;
    if (taskId === null) {
      return;
    }
    try {
      await invoke('cancel_streaming_task', { taskId });
    } finally {
      activeTaskId.value = null;
      messages.value = messages.value.map(m => (m.status === 'streaming' ? { ...m, status: 'error' } : m));
    }
  };

  /**
   * 按 messageId 流转 assistant 消息状态：流式 finished -> completed、error/超时 -> error。
   * 由 StreamableAudioPlayer 经 emit 回调驱动，补齐「初版不追踪流式结束」的缺口，
   * 使已完成消息不被 terminateSession 误标为 error。
   */
  const updateMessageStatus = (messageId: string, status: StreamingChatMessage['status']) => {
    messages.value = messages.value.map(m => (m.id === messageId ? { ...m, status } : m));
  };

  const clearMessages = () => {
    messages.value = [];
    activeTaskId.value = null;
  };

  return {
    speakers,
    messages,
    isDrawerOpen,
    activeTaskId,
    isStartingSession,
    sessionConfig,
    speakerOptions,
    getSpeaker,
    addSpeaker,
    updateSpeaker,
    removeSpeaker,
    openDrawer,
    closeDrawer,
    toggleDrawer,
    setSessionConfig,
    sendMessage,
    terminateSession,
    updateMessageStatus,
    clearMessages
  };
});
