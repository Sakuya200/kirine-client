import { APP_LANGUAGE_LABELS, AppLanguage } from './language';

export enum TextToSpeechFormat {
  Wav = 'wav',
  Mp3 = 'mp3',
  Flac = 'flac'
}

export interface TextToSpeechOption {
  label: string;
  value: string | number;
}

export interface TextToSpeechSpeakerOption extends TextToSpeechOption {
  description: string;
}

export const TEXT_TO_SPEECH_LANGUAGES: TextToSpeechOption[] = [
  { value: AppLanguage.Chinese, label: APP_LANGUAGE_LABELS[AppLanguage.Chinese] },
  { value: AppLanguage.English, label: APP_LANGUAGE_LABELS[AppLanguage.English] },
  { value: AppLanguage.Japanese, label: APP_LANGUAGE_LABELS[AppLanguage.Japanese] }
];

export const TEXT_TO_SPEECH_FORMATS: TextToSpeechOption[] = [
  { value: TextToSpeechFormat.Wav, label: 'WAV 无损' },
  { value: TextToSpeechFormat.Mp3, label: 'MP3 压缩' },
  { value: TextToSpeechFormat.Flac, label: 'FLAC 无损压缩' }
];
