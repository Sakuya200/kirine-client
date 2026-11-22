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
    // 元素只创建一次并持续复用：src 变化（流式播放中 Blob URL 节流更新）时直接换 src
    // 触发重载，事件监听保持不变；仅在 destroyAudioElement（reset/releaseSource）时销毁，
    // 消灭逐 chunk「销毁 + new Audio + 重挂监听」的重建风暴。
    if (!audioElement) {
      const next = new Audio();

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
    const element = audioElement;
    if (element.src !== src) {
      element.src = src;
    }
    const resumeTime = playbackOptions.resumeTime ?? 0;
    if (resumeTime > 0 && Number.isFinite(resumeTime)) {
      // 换 src 场景：新媒体尚未加载（duration 为 NaN），按规范记录默认起播位置；
      // 同 src 场景：currentTime 赋值为自身，无副作用。
      element.currentTime = Math.min(resumeTime, element.duration || resumeTime);
      audioCurrentTime.value = element.currentTime;
    }
    if (playbackOptions.autoplay) {
      element.play().catch(() => options.onPlaybackError?.());
    }
    return element;
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
