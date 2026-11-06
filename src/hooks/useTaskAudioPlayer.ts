import { computed, onBeforeUnmount, ref } from 'vue';

import { TaskStatus } from '@/enums/status';

interface PlaybackTaskResult {
  taskId: number;
  status: TaskStatus;
}

interface UseTaskAudioPlayerOptions {
  /** 解析任务的播放 URL（asset 协议地址）：invoke get_generated_audio_path + convertFileSrc。 */
  loadAudioUrl: (taskId: number) => Promise<string>;
  onPlaybackEnded?: () => void;
  onPlaybackError?: () => void;
  onPlayFailed?: (error: unknown) => void;
}

export const useTaskAudioPlayer = <Result extends PlaybackTaskResult>(options: UseTaskAudioPlayerOptions) => {
  const isPlaying = ref(false);
  const audioCurrentTime = ref(0);
  const audioDuration = ref(0);
  const playbackProgress = ref(0);

  let audioElement: HTMLAudioElement | null = null;
  let audioUrl: string | null = null;
  let audioUrlTaskId: number | null = null;
  let removeAudioListeners: (() => void) | null = null;

  const currentPlaybackSeconds = computed(() => Math.round(audioCurrentTime.value));
  const playbackTotalSeconds = computed(() => Math.round(audioDuration.value || 0));

  const syncAudioProgress = () => {
    if (!audioElement) {
      audioCurrentTime.value = 0;
      audioDuration.value = 0;
      playbackProgress.value = 0;
      return;
    }

    const duration = Number.isFinite(audioElement.duration) && audioElement.duration > 0 ? audioElement.duration : 0;
    audioDuration.value = duration;
    audioCurrentTime.value = audioElement.currentTime;
    playbackProgress.value = duration > 0 ? Math.min(100, (audioElement.currentTime / duration) * 100) : 0;
  };

  const resetAudioSource = () => {
    audioUrl = null;
    audioUrlTaskId = null;
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
  };

  const stopPlayback = () => {
    audioElement?.pause();
    isPlaying.value = false;
  };

  const resetPlayback = ({ releaseSource = false } = {}) => {
    destroyAudioElement();

    if (releaseSource) {
      resetAudioSource();
    }

    audioCurrentTime.value = 0;
    audioDuration.value = 0;
    playbackProgress.value = 0;
  };

  // asset 协议 URL 由 WebView 流式读盘，无 Blob/字节拷贝，也无需 revoke
  const getAudioSourceUrl = async (result: Result): Promise<string> => {
    if (audioUrl && audioUrlTaskId === result.taskId) {
      return audioUrl;
    }

    const url = await options.loadAudioUrl(result.taskId);
    resetAudioSource();
    audioUrl = url;
    audioUrlTaskId = result.taskId;

    return url;
  };

  const ensureAudioElement = async (result: Result) => {
    const url = await getAudioSourceUrl(result);

    if (!audioElement || audioElement.src !== url) {
      destroyAudioElement();
      audioCurrentTime.value = 0;
      audioDuration.value = 0;
      playbackProgress.value = 0;

      const nextAudioElement = new Audio(url);

      const handleTimeUpdate = () => syncAudioProgress();
      const handleLoadedMetadata = () => syncAudioProgress();
      const handlePause = () => {
        isPlaying.value = false;
        syncAudioProgress();
      };
      const handlePlay = () => {
        isPlaying.value = true;
        syncAudioProgress();
      };
      const handleEnded = () => {
        isPlaying.value = false;
        syncAudioProgress();
        options.onPlaybackEnded?.();
      };
      const handleError = () => {
        stopPlayback();
        options.onPlaybackError?.();
      };

      nextAudioElement.addEventListener('timeupdate', handleTimeUpdate);
      nextAudioElement.addEventListener('loadedmetadata', handleLoadedMetadata);
      nextAudioElement.addEventListener('pause', handlePause);
      nextAudioElement.addEventListener('play', handlePlay);
      nextAudioElement.addEventListener('ended', handleEnded);
      nextAudioElement.addEventListener('error', handleError);

      removeAudioListeners = () => {
        nextAudioElement.removeEventListener('timeupdate', handleTimeUpdate);
        nextAudioElement.removeEventListener('loadedmetadata', handleLoadedMetadata);
        nextAudioElement.removeEventListener('pause', handlePause);
        nextAudioElement.removeEventListener('play', handlePlay);
        nextAudioElement.removeEventListener('ended', handleEnded);
        nextAudioElement.removeEventListener('error', handleError);
      };

      audioElement = nextAudioElement;
    }

    return audioElement;
  };

  const playResultAudio = async (result: Result) => {
    if (result.status !== TaskStatus.Completed) {
      return false;
    }

    const targetAudioElement = await ensureAudioElement(result);

    try {
      await targetAudioElement.play();
      return true;
    } catch (error) {
      options.onPlayFailed?.(error);
      return false;
    }
  };

  const togglePlayback = (result: Result | null) => {
    if (!result) {
      return false;
    }

    if (result.status !== TaskStatus.Completed) {
      return false;
    }

    if (isPlaying.value) {
      stopPlayback();
      return false;
    }

    void playResultAudio(result);
    return true;
  };

  onBeforeUnmount(() => {
    resetPlayback({ releaseSource: true });
  });

  return {
    isPlaying,
    audioCurrentTime,
    audioDuration,
    playbackProgress,
    currentPlaybackSeconds,
    playbackTotalSeconds,
    playResultAudio,
    togglePlayback,
    stopPlayback,
    resetPlayback
  };
};
