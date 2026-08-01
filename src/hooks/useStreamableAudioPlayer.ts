import { computed, onBeforeUnmount, ref, watch, type Ref } from 'vue';
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
  const localHasData = ref(false);
  const hasData = options.hasData ?? localHasData;

  const currentPlaybackSeconds = computed(() => Math.round(audioCurrentTime.value));
  const currentSourceUrl = computed(() => options.sourceUrl?.() ?? pathUrl);

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

  const ensureAudioElement = (src: string, playbackOptions: { autoplay?: boolean; resumeTime?: number } = {}) => {
    const shouldRecreate = !audioElement || audioElement.src !== src;
    if (shouldRecreate) {
      const resumeTime = playbackOptions.resumeTime ?? 0;
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
      if (resumeTime > 0 && Number.isFinite(resumeTime)) {
        next.currentTime = Math.min(resumeTime, next.duration || resumeTime);
        audioCurrentTime.value = next.currentTime;
      }
    }
    if (playbackOptions.autoplay && audioElement) {
      audioElement.play().catch(() => options.onPlaybackError?.());
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
    const resumeTime = audioElement?.currentTime ?? audioCurrentTime.value;
    const element = ensureAudioElement(src, {
      autoplay: true,
      resumeTime: Number.isFinite(resumeTime) ? resumeTime : 0
    });
    if (!element) {
      options.onPlaybackError?.();
    }
  };

  const startPlayback = () => {
    if (!hasData.value || isPlaying.value) {
      return false;
    }
    const src = options.sourceUrl?.() ?? pathUrl;
    if (!src) {
      return false;
    }
    const resumeTime = audioElement?.currentTime ?? audioCurrentTime.value;
    const element = ensureAudioElement(src, {
      autoplay: true,
      resumeTime: Number.isFinite(resumeTime) ? resumeTime : 0
    });
    if (!element) {
      options.onPlaybackError?.();
      return false;
    }
    return true;
  };

  const setAudioPath = (filePath: string) => {
    destroyAudioElement();
    pathUrl = filePath ? convertFileSrc(filePath) : null;
    localHasData.value = !!pathUrl;
  };

  watch(currentSourceUrl, (nextSource, previousSource) => {
    if (!nextSource || !isPlaying.value || nextSource === previousSource) {
      return;
    }
    const currentTime = audioElement?.currentTime ?? 0;
    const element = ensureAudioElement(nextSource, {
      autoplay: true,
      resumeTime: currentTime
    });
    if (!element) {
      options.onPlaybackError?.();
    }
  });

  onBeforeUnmount(() => {
    reset({ releaseSource: true });
  });

  return {
    isPlaying,
    hasData,
    playbackProgress,
    currentPlaybackSeconds,
    setAudioPath,
    startPlayback,
    togglePlayback,
    stopPlayback,
    reset
  };
};
