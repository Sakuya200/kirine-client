import { HistoryTaskType } from '@/enums/task';

const padTimestampPart = (value: number) => String(value).padStart(2, '0');

const formatTimestamp = (date: Date) =>
  [date.getFullYear(), padTimestampPart(date.getMonth() + 1), padTimestampPart(date.getDate())].join('') +
  '_' +
  [padTimestampPart(date.getHours()), padTimestampPart(date.getMinutes()), padTimestampPart(date.getSeconds())].join('');

export const createTaskExportAudioName = (taskType: HistoryTaskType, date: Date = new Date()) => `${taskType}_${formatTimestamp(date)}`;
