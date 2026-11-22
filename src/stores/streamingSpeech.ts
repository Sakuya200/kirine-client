import { defineStore } from 'pinia';
import { computed, reactive, ref } from 'vue';
import { Channel, convertFileSrc, invoke } from '@tauri-apps/api/core';
import { i18n } from '@/locales';

import { AppLanguage } from '@/enums/language';
import { HardwareType } from '@/enums/settings';
import { useUiStore } from '@/stores/ui';
import type { StreamingReplaySnapshot, StreamingSpeechTaskResult } from '@/types/domain';
import type {
  StreamingChatMessage,
  StreamingSessionConfig,
  StreamingSpeakerCategory,
  StreamingSpeakerConfig,
  StreamingSpeakerSide
} from '@/types/streaming';

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
  /** 消息展示侧，缺省视为 right。 */
  side?: StreamingSpeakerSide;
  /** 头像原图绝对路径（仅会话覆盖时有值；记录头像经 speakerId 由后端按需读取）。 */
  avatarPath?: string;
  /** 头像原始文件名（展示用）。 */
  avatarName?: string;
  /** speakers 表记录 id（trained/预置说话人记录来源，后端记录头像回退用）。 */
  speakerId?: number | null;
  /** 记录头像 MIME 类型（表单内部状态：有值且无 avatarPath 覆盖 = 使用记录头像）。 */
  avatarContentType?: string | null;
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
type AudioStreamEvent = { type: 'started' } | { type: 'finished' } | { type: 'error'; message: string } | ArrayBuffer;

let speakerSeed = 0;
let messageSeed = 0;
const nextSpeakerId = () => `spk-${++speakerSeed}`;
const nextMessageId = () => `msg-${++messageSeed}`;

/** 说话人配置 → 后端 StreamingSpeakerInput 映射（startSession 与 persistSpeakers 共用）。 */
const toSpeakerPayload = (s: StreamingSpeakerConfig) => ({
  name: s.name,
  baseModel: s.baseModel,
  modelVersion: s.modelVersion,
  refAudioPath: s.refAudioPath,
  refAudioName: s.refAudioName,
  refText: s.refText,
  description: s.description,
  category: s.category,
  speakerDirName: s.speakerDirName,
  side: s.side,
  avatarPath: s.avatarPath,
  avatarName: s.avatarName,
  speakerId: s.speakerId ?? null
});

export const useStreamingSpeechStore = defineStore('streaming-speech', () => {
  const uiStore = useUiStore();
  const speakers = ref<StreamingSpeakerConfig[]>([]);
  const messages = ref<StreamingChatMessage[]>([]);
  const isDrawerOpen = ref(false);
  const activeTaskId = ref<number | null>(null);
  /** 回放模式下的历史任务 id（回放不占用 activeTaskId，但头像同步仍需定位任务）。 */
  const replayTaskId = ref<number | null>(null);
  const isStartingSession = ref(false);
  const sessionConfig = reactive<StreamingSessionConfig>({
    baseModel: '',
    modelVersion: '',
    device: HardwareType.Cpu,
    language: AppLanguage.Chinese,
    modelParams: {}
  });
  const audioStates = reactive<Record<string, AudioState>>({});
  /**
   * 每条消息的音频分块（chunk 到达仅 push 引用，聚合交给 Blob，避免逐 chunk 全量拷贝）。
   * 刻意保持非响应式：deep-reactive 包装会让每个 ~12KB chunk 的 push 触发依赖追踪开销，
   * 重建时机统一由 audioBuffersVersion 驱动。
   */
  const audioBuffers = new Map<string, Uint8Array[]>();
  /** chunk 到达计数器：递增即驱动 getAudioUrl 所在 computed 重算（Blob 重建受节流控制）。 */
  const audioBuffersVersion = ref(0);
  /** 流式进行中每累计多少个 chunk 才重建一次 Blob URL（~4 chunk/s，约 1.5s 一次）。 */
  const AUDIO_URL_REBUILD_CHUNK_INTERVAL = 6;
  /**
   * 消息 → 播放 URL 缓存。version = 构建时的 audioBuffersVersion（-1 哨兵 = 路径 URL）；
   * chunkCount = 构建时的 chunk 数量（节流比较用）。
   */
  const audioUrls = ref(new Map<string, { url: string; version: number; chunkCount: number }>());

  const speakerOptions = computed(() =>
    speakers.value.map(speaker => ({
      label: speaker.name,
      value: speaker.id,
      description: speaker.description || i18n.global.t('streaming.store.speakerDescriptionFallback', { name: speaker.refAudioName || i18n.global.t('streaming.drawer.noRefAudio') })
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
      if (cached.version !== -1) {
        URL.revokeObjectURL(cached.url);
      }
      audioUrls.value.delete(messageId);
    }
  };

  const clearAudioForMessage = (messageId: string) => {
    revokeAudioUrl(messageId);
    audioBuffers.delete(messageId);
    // 清空后版本递增，确保引用该消息 URL 的 computed 立即失效
    audioBuffersVersion.value += 1;
    delete audioStates[messageId];
  };

  const getAudioUrl = (messageId: string): string | null => {
    // 读取版本号建立响应式依赖：chunk 到达递增版本驱动重算；
    // 重算本身廉价（无 Blob 聚合），真正的 Blob 重建受下方节流控制。
    const version = audioBuffersVersion.value;
    const state = audioStates[messageId];
    if (!state?.hasData) {
      return null;
    }
    const cached = audioUrls.value.get(messageId);
    if (cached && cached.version === -1) {
      // 路径 URL（convertFileSrc）：静态资源，无需节流或重建
      return cached.url;
    }
    const chunks = audioBuffers.get(messageId);
    if (!chunks || chunks.length === 0) {
      return cached?.version === -1 ? cached.url : null;
    }
    if (cached && cached.version === version) {
      return cached.url;
    }
    // 节流：流式进行中每累计 ~6 个 chunk（约 1.5s）才重建一次 Blob URL，
    // 消灭逐 chunk O(n²) 聚合；流结束（isStreaming=false）立即重建完整音频。
    if (state.isStreaming && cached && chunks.length - cached.chunkCount < AUDIO_URL_REBUILD_CHUNK_INTERVAL) {
      return cached.url;
    }
    revokeAudioUrl(messageId);
    // Blob 直接接受分块数组聚合，无需先拼接成单个缓冲
    const blob = new Blob(chunks as BlobPart[], { type: 'audio/wav' });
    const url = URL.createObjectURL(blob);
    audioUrls.value.set(messageId, { url, version, chunkCount: chunks.length });
    return url;
  };

  const setAudioPathForMessage = (messageId: string, audioPath: string | null) => {
    revokeAudioUrl(messageId);
    audioBuffers.delete(messageId);

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
    audioUrls.value.set(messageId, { url: convertFileSrc(audioPath), version: -1, chunkCount: 0 });
  };

  /** 说话人列表持久化进行中标记（乐观更新已生效、后端写入未完成）。 */
  const persistingSpeakers = ref(false);

  /**
   * 将当前说话人列表全量写入任务 context.json；会话运行中（activeTaskId）时后端
   * 还会经 0x11 帧通知 Python 进程热重建说话人表，实现运行中即时生效。
   * 乐观更新由调用方先行完成，这里在失败时回滚到调用前快照。
   */
  const persistSpeakers = async (snapshot: StreamingSpeakerConfig[]) => {
    const taskId = activeTaskId.value ?? replayTaskId.value;
    if (taskId === null) {
      return;
    }
    persistingSpeakers.value = true;
    try {
      await invoke('update_streaming_speakers', {
        payload: {
          historyId: taskId,
          speakers: speakers.value.map(toSpeakerPayload)
        }
      });
    } catch (error) {
      speakers.value = snapshot;
      uiStore.notifyError(i18n.global.t('streaming.store.syncSpeakersFailed', { message: error instanceof Error ? error.message : String(error) }));
    } finally {
      persistingSpeakers.value = false;
    }
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
      description: payload.description,
      side: payload.side ?? 'right',
      avatarPath: payload.avatarPath,
      avatarName: payload.avatarName,
      speakerId: payload.speakerId ?? null
    };
    const snapshot = speakers.value;
    speakers.value = [...speakers.value, speaker];
    void persistSpeakers(snapshot);
    return speaker;
  };

  const updateSpeaker = (id: string, patch: Partial<StreamingSpeakerInput>) => {
    const snapshot = speakers.value;
    speakers.value = speakers.value.map(item =>
      item.id === id
        ? {
            ...item,
            // name 即说话人身份（context.json id = name），不支持改名
            ...patch,
            name: item.name,
            id: item.id,
            category: item.category
          }
        : item
    );
    void persistSpeakers(snapshot);
  };

  const removeSpeaker = (id: string) => {
    const snapshot = speakers.value;
    speakers.value = speakers.value.filter(item => item.id !== id);
    void persistSpeakers(snapshot);
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
      throw new Error(i18n.global.t('streaming.store.addSpeakerFirst'));
    }
    if (!sessionConfig.baseModel) {
      throw new Error(i18n.global.t('streaming.store.selectModelFirst'));
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
          speakers: speakers.value.map(toSpeakerPayload)
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
      uiStore.notifyWarning(i18n.global.t('streaming.store.startSessionFirst'));
      return;
    }

    // 每个说话人即用户本身：每次发送只产生一条消息，contextId 即消息 id
    // （后端以其命名 audio/<contextId>.wav）。
    const messageId = nextMessageId();
    const message: StreamingChatMessage = {
      id: messageId,
      text: trimmed,
      speakerId,
      speakerName: speaker?.name,
      taskId,
      contextId: messageId,
      status: 'streaming'
    };
    messages.value = [...messages.value, message];

    const state = ensureAudioState(messageId);
    state.isStreaming = true;
    state.hasData = false;
    state.streamComplete = false;
    state.errorMessage = null;
    audioBuffers.delete(messageId);
    revokeAudioUrl(messageId);

    const channel = new Channel<AudioStreamEvent>();
    channel.onmessage = (message: AudioStreamEvent) => {
      // chunk 为二进制载荷（ArrayBuffer）；控制事件为 JSON 对象
      if (message instanceof ArrayBuffer) {
        state.isStreaming = true;
        state.hasData = true;
        let chunks = audioBuffers.get(messageId);
        if (!chunks) {
          chunks = [];
          audioBuffers.set(messageId, chunks);
        }
        chunks.push(new Uint8Array(message));
        // 非响应式 chunk 存储的统一失效信号：驱动 getAudioUrl 重算（重建受节流控制）
        audioBuffersVersion.value += 1;
        return;
      }
      switch (message.type) {
        case 'started':
          state.isStreaming = true;
          break;
        case 'finished': {
          state.isStreaming = false;
          state.streamComplete = true;
          // 流结束立即触发最终重建（节流仅在 isStreaming 时生效）
          audioBuffersVersion.value += 1;
          updateMessageStatus(messageId, 'completed');
          break;
        }
        case 'error': {
          state.isStreaming = false;
          state.streamComplete = false;
          state.errorMessage = message.message;
          updateMessageStatus(messageId, 'error');
          uiStore.notifyError(i18n.global.t('streaming.store.audioStreamFailed', { message: message.message }));
          break;
        }
      }
    };

    try {
      await invoke('send_streaming_message', {
        payload: { taskId, contextId: messageId, speakerName: speaker?.name ?? '', text: trimmed },
        onEvent: channel
      });
    } catch (error) {
      state.isStreaming = false;
      state.streamComplete = false;
      state.errorMessage = error instanceof Error ? error.message : String(error);
      updateMessageStatus(messageId, 'error');
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
    replayTaskId.value = null;
  };

  const restoreFromReplaySnapshot = (snapshot: StreamingReplaySnapshot) => {
    for (const message of messages.value) {
      clearAudioForMessage(message.id);
    }
    replayTaskId.value = snapshot.taskId;

    speakers.value = snapshot.speakers.map(
      speaker =>
        ({
          id: nextSpeakerId(),
          category: speaker.category ?? 'voice-clone',
          speakerDirName: speaker.speakerDirName,
          name: speaker.name,
          baseModel: speaker.baseModel,
          modelVersion: speaker.modelVersion,
          refAudioPath: speaker.refAudioPath,
          refAudioName: speaker.refAudioName,
          refText: speaker.refText,
          description: speaker.description,
          side: speaker.side ?? 'right',
          avatarPath: speaker.avatarPath,
          avatarName: speaker.avatarName,
          speakerId: speaker.speakerId ?? null
        }) as StreamingSpeakerConfig
    );

    setSessionConfig({
      baseModel: snapshot.baseModel,
      modelVersion: snapshot.modelVersion,
      device: snapshot.device,
      language: snapshot.language,
      modelParams: snapshot.modelParams ?? {}
    });

    messages.value = [];
    for (const entry of snapshot.messages) {
      const messageId = entry.messageId ?? entry.contextId;
      const matchedSpeaker = speakers.value.find(speaker => speaker.name === entry.speakerName) ?? null;

      const message: StreamingChatMessage = {
        id: messageId,
        text: entry.text,
        speakerId: matchedSpeaker?.id ?? null,
        speakerName: entry.speakerName,
        taskId: snapshot.taskId,
        contextId: messageId,
        audioPath: entry.audioPath,
        status: 'completed'
      };

      messages.value.push(message);
      setAudioPathForMessage(messageId, entry.audioPath ?? null);
    }
  };

  return {
    speakers,
    messages,
    isDrawerOpen,
    activeTaskId,
    replayTaskId,
    isStartingSession,
    sessionConfig,
    audioStates,
    speakerOptions,
    getSpeaker,
    getMessage,
    addSpeaker,
    updateSpeaker,
    removeSpeaker,
    persistingSpeakers,
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
