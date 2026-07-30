import { computed, onBeforeUnmount, ref, type Ref } from 'vue';
import { convertFileSrc } from '@tauri-apps/api/core';

interface UseStreamableAudioPlayerOptions {
  sourceUrl?: () => string | null;
  hasData?: Ref<boolean>;
  onPlaybackEnded?: () => void;
  onPlaybackError?: () => void;
}

export const useStreamableAudioPlayer = (options: UseStreamableAudioPlayerOptions = {}) => {
  const isPlaying = ref(false);
  const playbackProgress = ref(0);
  const audioCurrentTime = ref(0);
  const hasData = options.hasData ?? ref(false);

  const currentPlaybackSeconds = computed(() => Math.round(audioCurrentTime.value));

  let audioElement: HTMLAudioElement | null = null;
  let pathUrl: string | null = null;
  let removeAudioListeners: (() => void) | null = null;

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
      pathUrl = null;
    }
  };

  const ensureAudioElement = (src: string) => {
    if (!audioElement || audioElement.src !== src) {
      destroyAudioElement();
      const next = new Audio(src);

      const handleTimeUpdate = () => {
        const duration = Number.isFinite(next.duration) && next.duration > 0 ? next.duration : 0;
        audioCurrentTime.value = next.currentTime;
        playbackProgress.value = duration > 0 ? Math.min(100, (next.currentTime / duration) * 100) : 0;
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

  const togglePlayback = () => {
    if (!hasData.value) {
      return;
    }
    if (isPlaying.value) {
      stopPlayback();
      return;
    }
    const src = options.sourceUrl?.() ?? pathUrl;
    if (!src) {
      return;
    }
    const element = ensureAudioElement(src);
    element.play().catch(() => options.onPlaybackError?.());
  };

  const setAudioPath = (filePath: string) => {
    destroyAudioElement();
    pathUrl = filePath ? convertFileSrc(filePath) : null;
    hasData.value = !!pathUrl;
  };

  onBeforeUnmount(() => {
    reset({ releaseSource: true });
  });

  return {
    isPlaying,
    hasData,
    playbackProgress,
    currentPlaybackSeconds,
    setAudioPath,
    togglePlayback,
    stopPlayback,
    reset
  };
};
