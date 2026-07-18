import { defineStore } from 'pinia';
import { computed, reactive, ref } from 'vue';

import { AppLanguage } from '@/enums/language';
import { HardwareType } from '@/enums/settings';
import type { StreamingChatMessage, StreamingSessionConfig, StreamingSpeakerConfig } from '@/types/streaming';

/**
 * 需要参考文本的模型集合（对齐 TextToSpeechView 的 DYNAMIC_REFERENCE_BASE_MODELS）。
 * 后续可由后端模型能力字段驱动，本期以前端常量判断。
 */
export const REQUIRES_REF_TEXT_MODELS = new Set(['gpt_sovits_cpufast']);
export const requiresRefText = (baseModel: string) => REQUIRES_REF_TEXT_MODELS.has(baseModel);

/** 新增/编辑说话人时由表单提交的输入（不含 id/category/时间，时间由后端生成）。 */
export interface StreamingSpeakerInput {
  name: string;
  baseModel: string;
  modelVersion?: string;
  refAudioPath: string;
  refAudioName: string;
  refText: string;
  description?: string;
}

let speakerSeed = 0;
let messageSeed = 0;
let taskSeed = 0;
const nextSpeakerId = () => `spk-${++speakerSeed}`;
const nextMessageId = () => `msg-${++messageSeed}`;
const nextTaskId = () => ++taskSeed;

export const useStreamingSpeechStore = defineStore('streaming-speech', () => {
  const speakers = ref<StreamingSpeakerConfig[]>([]);
  const messages = ref<StreamingChatMessage[]>([]);
  const isDrawerOpen = ref(false);
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
      category: 'voice-clone',
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
   * 发送一条聊天消息：push user 消息 + 占位 assistant 消息。
   * 实际流式接收由渲染出的 StreamableAudioPlayer(mode='stream') 在 watch immediate 时
   * 自动调 startStreaming(taskId, contextId) 完成，符合既有组件契约；此处不 invoke。
   * 提交后端的 payload 不含时间字段，任务创建时间由后端执行前生成。
   */
  const sendMessage = (text: string, speakerId: string | null) => {
    const trimmed = text.trim();
    if (trimmed.length === 0) {
      return;
    }

    const speaker = getSpeaker(speakerId);
    const userMessage: StreamingChatMessage = {
      id: nextMessageId(),
      role: 'user',
      text: trimmed,
      speakerId,
      speakerName: speaker?.name,
      taskId: 0,
      contextId: '',
      status: 'completed'
    };
    messages.value = [...messages.value, userMessage];

    const assistantId = nextMessageId();
    const assistantMessage: StreamingChatMessage = {
      id: assistantId,
      role: 'assistant',
      text: '',
      speakerId,
      speakerName: speaker?.name,
      taskId: nextTaskId(),
      contextId: assistantId,
      status: 'streaming'
    };
    messages.value = [...messages.value, assistantMessage];
  };

  const clearMessages = () => {
    messages.value = [];
  };

  return {
    speakers,
    messages,
    isDrawerOpen,
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
    clearMessages
  };
});
