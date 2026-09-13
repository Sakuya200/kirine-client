export enum HistoryTaskType {
  ModelTraining = 'model-training',
  TextToSpeech = 'text-to-speech',
  VoiceClone = 'voice-clone',
  VoiceDesign = 'voice-design',
  StreamingSpeech = 'streaming-speech'
}

export const HISTORY_TASK_ROUTE_PATH: Record<HistoryTaskType, string> = {
  [HistoryTaskType.ModelTraining]: `/${HistoryTaskType.ModelTraining}`,
  [HistoryTaskType.TextToSpeech]: `/${HistoryTaskType.TextToSpeech}`,
  [HistoryTaskType.VoiceClone]: `/${HistoryTaskType.VoiceClone}`,
  [HistoryTaskType.VoiceDesign]: `/${HistoryTaskType.VoiceDesign}`,
  [HistoryTaskType.StreamingSpeech]: '/streaming-speech'
};

export const HISTORY_TASK_REPLAY_QUERY_KEY = 'replayTaskId';

export const getHistoryTaskReplayId = (value: string | null | Array<string | null> | undefined) => {
  const rawValue = Array.isArray(value) ? value[0] : value;

  if (typeof rawValue !== 'string') {
    return null;
  }

  const historyId = Number.parseInt(rawValue, 10);
  return Number.isSafeInteger(historyId) && historyId > 0 ? historyId : null;
};

export const HISTORY_TASK_TYPE_TEXT_KEY: Record<HistoryTaskType, string> = {
  [HistoryTaskType.ModelTraining]: 'common.historyTaskType.modelTraining',
  [HistoryTaskType.TextToSpeech]: 'common.historyTaskType.textToSpeech',
  [HistoryTaskType.VoiceClone]: 'common.historyTaskType.voiceClone',
  [HistoryTaskType.VoiceDesign]: 'common.historyTaskType.voiceDesign',
  [HistoryTaskType.StreamingSpeech]: 'common.historyTaskType.streamingSpeech'
};
