import { invoke } from '@tauri-apps/api/core';

export type GeneratedAudioDownloadSource =
  | { kind: 'text-to-speech'; historyId: number }
  | { kind: 'voice-clone'; historyId: number }
  | { kind: 'voice-design'; historyId: number }
  | { kind: 'streaming-speech'; historyId: number; messageId: string };

export const saveGeneratedAudio = (source: GeneratedAudioDownloadSource) =>
  invoke<boolean>('save_generated_audio_as', {
    source
  });
