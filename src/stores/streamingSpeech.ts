import { defineStore } from 'pinia';
import { computed, reactive, ref } from 'vue';
import { Channel, convertFileSrc, invoke } from '@tauri-apps/api/core';

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
  /** 头像原图绝对路径（创建任务时由后端复制进任务 sample 目录）。 */
  avatarPath?: string;
  /** 头像原始文件名（展示用）。 */
  avatarName?: string;
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
  avatarName: s.avatarName
});

export const useStreamingSpeechStore = defineStore('streaming-speech', () => {
  const uiStore = useUiStore();
  const speakers = ref<StreamingSpeakerConfig[]>([]);
  const messages = ref<StreamingChatMessage[]>([]);
  const isDrawerOpen = ref(false);
  const activeTaskId = ref<number | null>(null);
  /** 回放模式下的历史任务 id（回放不占用 activeTaskId，但头像同步仍需定位任务）。 */
  const replayTaskId = ref<number | null>(null);
  /** 头像缓存版本：clearAvatarUrls 递增，驱动已挂载的头像组件 watch 重载（替代旧的 messagesVersion 整列表重挂载）。 */
  const avatarCacheVersion = ref(0);
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
    const blob = new Blob(chunks as BlobPart[], { type: 'audio/wav' });
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

  /** 说话人头像 Blob URL 缓存：key 为 `${taskId}::${speakerName}`，值空串表示已确认无头像/加载失败。 */
  const avatarUrls = ref(new Map<string, string>());

  const clearAvatarUrls = () => {
    for (const url of avatarUrls.value.values()) {
      if (url) {
        URL.revokeObjectURL(url);
      }
    }
    avatarUrls.value.clear();
    // 缓存已整体失效，递增版本号让已挂载的头像组件 watch 重载
    avatarCacheVersion.value += 1;
  };

  const ensureSpeakerAvatar = async (taskId: number, speakerName: string): Promise<string | null> => {
    const key = `${taskId}::${speakerName}`;
    const cached = avatarUrls.value.get(key);
    if (cached !== undefined) {
      return cached || null;
    }
    try {
      const asset = await invoke<{ fileName: string; contentType: string; bytes: number[] }>('get_streaming_speaker_avatar', {
        historyId: taskId,
        speakerName
      });
      const blob = new Blob([Uint8Array.from(asset.bytes)], { type: asset.contentType });
      const url = URL.createObjectURL(blob);
      avatarUrls.value.set(key, url);
      return url;
    } catch {
      // 失败缓存空串，避免多消息场景下重复 invoke
      avatarUrls.value.set(key, '');
      return null;
    }
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
      // 头像可能被替换/清空：失效缓存让已挂载头像组件重载（version 驱动 watch）
      clearAvatarUrls();
    } catch (error) {
      speakers.value = snapshot;
      uiStore.notifyError(`同步说话人配置失败：${error instanceof Error ? error.message : String(error)}`);
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
      avatarName: payload.avatarName
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
      uiStore.notifyWarning('请先开启会话');
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
    audioBuffers.value.delete(messageId);
    revokeAudioUrl(messageId);

    const channel = new Channel<AudioStreamEvent>();
    channel.onmessage = (message: AudioStreamEvent) => {
      // chunk 为二进制载荷（ArrayBuffer）；控制事件为 JSON 对象
      if (message instanceof ArrayBuffer) {
        state.isStreaming = true;
        state.hasData = true;
        let chunks = audioBuffers.value.get(messageId);
        if (!chunks) {
          chunks = [];
          audioBuffers.value.set(messageId, chunks);
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
          updateMessageStatus(messageId, 'completed');
          break;
        }
        case 'error': {
          state.isStreaming = false;
          state.streamComplete = false;
          state.errorMessage = message.message;
          updateMessageStatus(messageId, 'error');
          uiStore.notifyError(`音频流式接收失败：${message.message}`);
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
    clearAvatarUrls();
    messages.value = [];
    activeTaskId.value = null;
    replayTaskId.value = null;
  };

  const restoreFromReplaySnapshot = (snapshot: StreamingReplaySnapshot) => {
    for (const message of messages.value) {
      clearAudioForMessage(message.id);
    }
    clearAvatarUrls();
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
          avatarName: speaker.avatarName
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
    setAudioPathForMessage,
    ensureSpeakerAvatar,
    avatarCacheVersion
  };
});
