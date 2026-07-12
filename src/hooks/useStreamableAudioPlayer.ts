import { computed, onBeforeUnmount, ref } from 'vue';
import { Channel, convertFileSrc, invoke } from '@tauri-apps/api/core';

type AudioStreamEvent =
  | { type: 'started' }
  | { type: 'chunk'; bytes: number[] }
  | { type: 'finished' }
  | { type: 'error'; message: string };

interface UseStreamableAudioPlayerOptions {
  onPlaybackEnded?: () => void;
  onPlaybackError?: () => void;
  onStreamError?: (message: string) => void;
}

export const useStreamableAudioPlayer = (options: UseStreamableAudioPlayerOptions = {}) => {
  const isPlaying = ref(false);
  const hasData = ref(false);
  const isStreaming = ref(false);
  const streamComplete = ref(false);
  const playbackProgress = ref(0);
  const audioCurrentTime = ref(0);

  const currentPlaybackSeconds = computed(() => Math.round(audioCurrentTime.value));

  let audioElement: HTMLAudioElement | null = null;
  let audioObjectUrl: string | null = null;
  let accumulated: number[] = [];
  let pathUrl: string | null = null;
  let removeAudioListeners: (() => void) | null = null;
  let streamObjectUrlLength = 0;

  const releaseObjectUrl = () => {
    if (audioObjectUrl) {
      URL.revokeObjectURL(audioObjectUrl);
      audioObjectUrl = null;
    }
    streamObjectUrlLength = 0;
  };

  const destroyAudioElement = () => {
    if (audioElement) {
      removeAudioListeners?.();
      removeAudioListeners = null;
      audioElement.pause();
      audioElement.removeAttribute('src');
      audioElement = null;
    }
    isPlaying.value = false;
    playbackProgress.value = 0;
    audioCurrentTime.value = 0;
  };

  const stopPlayback = () => {
    audioElement?.pause();
    isPlaying.value = false;
  };

  const reset = ({ releaseSource = false } = {}) => {
    destroyAudioElement();
    if (releaseSource) {
      releaseObjectUrl();
      accumulated = [];
      pathUrl = null;
      hasData.value = false;
      isStreaming.value = false;
      streamComplete.value = false;
    }
  };

  const ensureAudioElement = (src: string) => {
    if (!audioElement || audioElement.src !== src) {
      destroyAudioElement();
      const next = new Audio(src);

      const handleTimeUpdate = () => {
        const duration =
          Number.isFinite(next.duration) && next.duration > 0 ? next.duration : 0;
        audioCurrentTime.value = next.currentTime;
        playbackProgress.value =
          duration > 0 ? Math.min(100, (next.currentTime / duration) * 100) : 0;
      };
      const handlePause = () => {
        isPlaying.value = false;
      };
      const handlePlay = () => {
        isPlaying.value = true;
      };
      const handleEnded = () => {
        isPlaying.value = false;
        playbackProgress.value = 100;
        options.onPlaybackEnded?.();
      };
      const handleError = () => {
        stopPlayback();
        options.onPlaybackError?.();
      };

      next.addEventListener('timeupdate', handleTimeUpdate);
      next.addEventListener('pause', handlePause);
      next.addEventListener('play', handlePlay);
      next.addEventListener('ended', handleEnded);
      next.addEventListener('error', handleError);

      removeAudioListeners = () => {
        next.removeEventListener('timeupdate', handleTimeUpdate);
        next.removeEventListener('pause', handlePause);
        next.removeEventListener('play', handlePlay);
        next.removeEventListener('ended', handleEnded);
        next.removeEventListener('error', handleError);
      };

      audioElement = next;
    }
    return audioElement;
  };

  const resolveSourceUrl = (): string | null => {
    if (pathUrl) {
      return pathUrl;
    }
    if (accumulated.length > 0) {
      if (audioObjectUrl && streamObjectUrlLength === accumulated.length) {
        return audioObjectUrl;
      }
      releaseObjectUrl();
      const blob = new Blob([Uint8Array.from(accumulated)], { type: 'audio/wav' });
      audioObjectUrl = URL.createObjectURL(blob);
      streamObjectUrlLength = accumulated.length;
      return audioObjectUrl;
    }
    return null;
  };

  const togglePlayback = () => {
    if (!hasData.value) {
      return;
    }
    if (isPlaying.value) {
      stopPlayback();
      return;
    }
    const src = resolveSourceUrl();
    if (!src) {
      return;
    }
    const element = ensureAudioElement(src);
    element.play().catch(() => options.onPlaybackError?.());
  };

  const startStreaming = async (taskId: number, contextId: string) => {
    accumulated = [];
    hasData.value = false;
    streamComplete.value = false;
    isStreaming.value = true;
    destroyAudioElement();
    releaseObjectUrl();
    pathUrl = null;

    const channel = new Channel<AudioStreamEvent>();
    channel.onmessage = (message: AudioStreamEvent) => {
      switch (message.type) {
        case 'started':
          break;
        case 'chunk':
          accumulated.push(...message.bytes);
          if (!hasData.value) {
            hasData.value = true;
          }
          break;
        case 'finished':
          streamComplete.value = true;
          isStreaming.value = false;
          break;
        case 'error':
          isStreaming.value = false;
          options.onStreamError?.(message.message);
          break;
      }
    };

    try {
      await invoke('stream_audio_placeholder', { taskId, contextId, onEvent: channel });
    } catch (error) {
      isStreaming.value = false;
      options.onStreamError?.(error instanceof Error ? error.message : String(error));
    }
  };

  const setAudioPath = (filePath: string) => {
    destroyAudioElement();
    releaseObjectUrl();
    accumulated = [];
    pathUrl = filePath ? convertFileSrc(filePath) : null;
    hasData.value = !!pathUrl;
  };

  onBeforeUnmount(() => {
    reset({ releaseSource: true });
  });

  return {
    isPlaying,
    hasData,
    isStreaming,
    streamComplete,
    playbackProgress,
    currentPlaybackSeconds,
    startStreaming,
    setAudioPath,
    togglePlayback,
    stopPlayback,
    reset,
  };
};
