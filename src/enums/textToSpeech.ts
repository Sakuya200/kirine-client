export enum TextToSpeechFormat {
  Wav = 'wav',
  Mp3 = 'mp3',
  Flac = 'flac'
}

export interface TextToSpeechOption {
  label: string;
  value: string | number | null;
}

export interface TextToSpeechSpeakerOption extends TextToSpeechOption {
  description: string;
}

export const TEXT_TO_SPEECH_FORMATS: TextToSpeechOption[] = [
  { value: TextToSpeechFormat.Wav, label: 'common.textToSpeech.format.wav' },
  { value: TextToSpeechFormat.Mp3, label: 'common.textToSpeech.format.mp3' },
  { value: TextToSpeechFormat.Flac, label: 'common.textToSpeech.format.flac' }
];
