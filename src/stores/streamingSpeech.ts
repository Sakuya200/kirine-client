import { defineStore } from 'pinia';
import { computed, reactive, ref } from 'vue';
import { Channel, convertFileSrc, invoke } from '@tauri-apps/api/core';

import { AppLanguage } from '@/enums/language';
import { HardwareType } from '@/enums/settings';
import { useUiStore } from '@/stores/ui';
import type { StreamingReplaySnapshot, StreamingSpeechTaskResult } from '@/types/domain';
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

interface AudioState {
  isStreaming: boolean;
  hasData: boolean;
  streamComplete: boolean;
  errorMessage: string | null;
}

/**
 * 流式音频事件：chunk 为二进制载荷（ArrayBuffer，Tauri Channel Raw 路径），
 * started/finished/error 为 JSON 控制事件。
 */
type AudioStreamEvent =
  | { type: 'started' }
  | { type: 'finished' }
  | { type: 'error'; message: string }
  | ArrayBuffer;

let speakerSeed = 0;
let messageSeed = 0;
const nextSpeakerId = () => `spk-${++speakerSeed}`;
const nextMessageId = () => `msg-${++messageSeed}`;

export const useStreamingSpeechStore = defineStore('streaming-speech', () => {
  const uiStore = useUiStore();
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
  const audioStates = reactive<Record<string, AudioState>>({});
  /** 每条消息的音频分块（chunk 到达仅 push 引用，聚合交给 Blob，避免逐 chunk 全量拷贝）。 */
  const audioBuffers = ref(new Map<string, Uint8Array[]>());
  const audioUrls = ref(new Map<string, { url: string; length: number }>());

  const speakerOptions = computed(() =>
    speakers.value.map(speaker => ({
      label: speaker.name,
      value: speaker.id,
      description: speaker.description || `参考音频：${speaker.refAudioName || '未设置'}`
    }))
  );

  const getSpeaker = (id: string | null) => speakers.value.find(item => item.id === id) ?? null;
  const getMessage = (id: string | null) => messages.value.find(item => item.id === id) ?? null;

  const ensureAudioState = (messageId: string): AudioState => {
    if (!audioStates[messageId]) {
      audioStates[messageId] = {
        isStreaming: false,
        hasData: false,
        streamComplete: false,
        errorMessage: null
      };
    }
    return audioStates[messageId];
  };

  const revokeAudioUrl = (messageId: string) => {
    const cached = audioUrls.value.get(messageId);
    if (cached) {
      if (cached.length !== -1) {
        URL.revokeObjectURL(cached.url);
      }
      audioUrls.value.delete(messageId);
    }
  };

  const clearAudioForMessage = (messageId: string) => {
    revokeAudioUrl(messageId);
    audioBuffers.value.delete(messageId);
    delete audioStates[messageId];
  };

  const getAudioUrl = (messageId: string): string | null => {
    const state = audioStates[messageId];
    if (!state?.hasData) {
      return null;
    }
    const cached = audioUrls.value.get(messageId);
    if (cached && cached.length === -1) {
      return cached.url;
    }
    const chunks = audioBuffers.value.get(messageId);
    if (!chunks || chunks.length === 0) {
      return cached?.length === -1 ? cached.url : null;
    }
    if (cached && cached.length === chunks.length) {
      return cached.url;
    }
    revokeAudioUrl(messageId);
    // Blob 直接接受分块数组聚合，无需先拼接成单个缓冲
    const blob = new Blob(chunks, { type: 'audio/wav' });
    const url = URL.createObjectURL(blob);
    audioUrls.value.set(messageId, { url, length: chunks.length });
    return url;
  };

  const setAudioPathForMessage = (messageId: string, audioPath: string | null) => {
    revokeAudioUrl(messageId);
    audioBuffers.value.delete(messageId);

    const state = ensureAudioState(messageId);
    if (!audioPath) {
      state.isStreaming = false;
      state.hasData = false;
      state.streamComplete = false;
      state.errorMessage = null;
      return;
    }

    state.isStreaming = false;
    state.hasData = true;
    state.streamComplete = true;
    state.errorMessage = null;
    audioUrls.value.set(messageId, { url: convertFileSrc(audioPath), length: -1 });
  };

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

  const startSession = async () => {
    if (activeTaskId.value !== null) {
      return;
    }
    if (speakers.value.length === 0) {
      throw new Error('请先在配置抽屉中添加说话人。');
    }
    if (!sessionConfig.baseModel) {
      throw new Error('请先选择模型。');
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
  };

  const sendMessage = async (text: string, speakerId: string | null) => {
    const trimmed = text.trim();
    if (trimmed.length === 0) {
      return;
    }
    if (isStartingSession.value) {
      return;
    }
    const speaker = getSpeaker(speakerId);
    const taskId = activeTaskId.value;
    if (taskId === null) {
      uiStore.notifyWarning('请先开启会话');
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

    const state = ensureAudioState(assistantId);
    state.isStreaming = true;
    state.hasData = false;
    state.streamComplete = false;
    state.errorMessage = null;
    audioBuffers.value.delete(assistantId);
    revokeAudioUrl(assistantId);

    const channel = new Channel<AudioStreamEvent>();
    channel.onmessage = (message: AudioStreamEvent) => {
      // chunk 为二进制载荷（ArrayBuffer）；控制事件为 JSON 对象
      if (message instanceof ArrayBuffer) {
        state.isStreaming = true;
        state.hasData = true;
        let chunks = audioBuffers.value.get(assistantId);
        if (!chunks) {
          chunks = [];
          audioBuffers.value.set(assistantId, chunks);
        }
        chunks.push(new Uint8Array(message));
        return;
      }
      switch (message.type) {
        case 'started':
          state.isStreaming = true;
          break;
        case 'finished': {
          state.isStreaming = false;
          state.streamComplete = true;
          updateMessageStatus(assistantId, 'completed');
          break;
        }
        case 'error': {
          state.isStreaming = false;
          state.streamComplete = false;
          state.errorMessage = message.message;
          updateMessageStatus(assistantId, 'error');
          uiStore.notifyError(`音频流式接收失败：${message.message}`);
          break;
        }
      }
    };

    try {
      await invoke('send_streaming_message', {
        payload: { taskId, contextId: assistantId, speakerName: speaker?.name ?? '', text: trimmed },
        onEvent: channel
      });
    } catch (error) {
      state.isStreaming = false;
      state.streamComplete = false;
      state.errorMessage = error instanceof Error ? error.message : String(error);
      updateMessageStatus(assistantId, 'error');
      uiStore.notifyError(error instanceof Error ? error.message : String(error));
    }
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

  const updateMessageStatus = (messageId: string, status: StreamingChatMessage['status']) => {
    messages.value = messages.value.map(m => (m.id === messageId ? { ...m, status } : m));
  };

  const clearMessages = () => {
    for (const message of messages.value) {
      clearAudioForMessage(message.id);
    }
    messages.value = [];
    activeTaskId.value = null;
  };

  const restoreFromReplaySnapshot = (snapshot: StreamingReplaySnapshot) => {
    for (const message of messages.value) {
      clearAudioForMessage(message.id);
    }

    speakers.value = snapshot.speakers.map(speaker => ({
      id: nextSpeakerId(),
      category: speaker.category ?? 'voice-clone',
      speakerDirName: speaker.speakerDirName,
      name: speaker.name,
      baseModel: speaker.baseModel,
      modelVersion: speaker.modelVersion,
      refAudioPath: speaker.refAudioPath,
      refAudioName: speaker.refAudioName,
      refText: speaker.refText,
      description: speaker.description
    }));

    setSessionConfig({
      baseModel: snapshot.baseModel,
      modelVersion: snapshot.modelVersion,
      device: snapshot.device,
      language: snapshot.language,
      modelParams: snapshot.modelParams ?? {}
    });

    messages.value = [];
    for (const entry of snapshot.messages) {
      const assistantMessageId = entry.messageId ?? entry.contextId;
      const matchedSpeaker = speakers.value.find(speaker => speaker.name === entry.speakerName) ?? null;

      const userMessage: StreamingChatMessage = {
        id: nextMessageId(),
        role: 'user',
        text: entry.text,
        synthText: entry.text,
        speakerId: matchedSpeaker?.id ?? null,
        speakerName: entry.speakerName,
        taskId: snapshot.taskId,
        contextId: `${assistantMessageId}-user`,
        status: 'completed'
      };
      const assistantMessage: StreamingChatMessage = {
        id: assistantMessageId,
        role: 'assistant',
        text: '',
        synthText: entry.text,
        speakerId: matchedSpeaker?.id ?? null,
        speakerName: entry.speakerName,
        taskId: snapshot.taskId,
        contextId: assistantMessageId,
        audioPath: entry.audioPath,
        status: 'completed'
      };

      messages.value.push(userMessage, assistantMessage);
      setAudioPathForMessage(assistantMessageId, entry.audioPath ?? null);
    }
  };

  return {
    speakers,
    messages,
    isDrawerOpen,
    activeTaskId,
    isStartingSession,
    sessionConfig,
    audioStates,
    speakerOptions,
    getSpeaker,
    getMessage,
    addSpeaker,
    updateSpeaker,
    removeSpeaker,
    openDrawer,
    closeDrawer,
    toggleDrawer,
    setSessionConfig,
    restoreFromReplaySnapshot,
    startSession,
    sendMessage,
    terminateSession,
    updateMessageStatus,
    clearMessages,
    getAudioUrl,
    setAudioPathForMessage
  };
});
