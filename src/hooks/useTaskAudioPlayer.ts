import { computed, onBeforeUnmount, ref } from 'vue';

import { TaskStatus } from '@/enums/status';

interface AudioAssetPayload {
  fileName: string;
  contentType: string;
  bytes: number[];
}

interface PlaybackTaskResult {
  taskId: number;
  status: TaskStatus;
}

interface UseTaskAudioPlayerOptions {
  loadAudioAsset: (taskId: number) => Promise<AudioAssetPayload>;
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
  let audioBlob: Blob | null = null;
  let audioObjectUrl: string | null = null;
  let audioObjectTaskId: number | null = null;
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

  const releaseAudioSource = () => {
    if (audioObjectUrl) {
      URL.revokeObjectURL(audioObjectUrl);
      audioObjectUrl = null;
    }

    audioBlob = null;
    audioObjectTaskId = null;
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
      releaseAudioSource();
    }

    audioCurrentTime.value = 0;
    audioDuration.value = 0;
    playbackProgress.value = 0;
  };

  const getAudioSource = async (result: Result) => {
    if (audioBlob && audioObjectUrl && audioObjectTaskId === result.taskId) {
      return {
        blob: audioBlob,
        objectUrl: audioObjectUrl
      };
    }

    const asset = await options.loadAudioAsset(result.taskId);
    const blob = new Blob([Uint8Array.from(asset.bytes)], {
      type: asset.contentType || 'audio/wav'
    });

    releaseAudioSource();
    audioBlob = blob;
    audioObjectUrl = URL.createObjectURL(blob);
    audioObjectTaskId = result.taskId;

    return {
      blob,
      objectUrl: audioObjectUrl
    };
  };

  const ensureAudioElement = async (result: Result) => {
    const asset = await getAudioSource(result);

    if (!audioElement || audioElement.src !== asset.objectUrl) {
      destroyAudioElement();
      audioCurrentTime.value = 0;
      audioDuration.value = 0;
      playbackProgress.value = 0;

      const nextAudioElement = new Audio(asset.objectUrl);

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
