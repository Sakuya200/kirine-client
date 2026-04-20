export enum TaskStatus {
  Pending = 'pending',
  Running = 'running',
  Completed = 'completed',
  Cancelled = 'cancelled',
  Failed = 'failed'
}

export enum SpeakerStatus {
  Ready = 'ready',
  Training = 'training',
  Disabled = 'disabled'
}

export const STATUS_TEXT: Record<TaskStatus, string> = {
  [TaskStatus.Pending]: '待执行',
  [TaskStatus.Running]: '执行中',
  [TaskStatus.Completed]: '已完成',
  [TaskStatus.Cancelled]: '已终止',
  [TaskStatus.Failed]: '失败'
};

export const STATUS_STYLES: Record<TaskStatus, string> = {
  [TaskStatus.Pending]: 'border-brand-200 bg-brand-50 text-brand-700',
  [TaskStatus.Running]: 'border-amber-300 bg-amber-50 text-amber-700',
  [TaskStatus.Completed]: 'border-emerald-300 bg-emerald-50 text-emerald-700',
  [TaskStatus.Cancelled]: 'border-slate-300 bg-slate-100 text-slate-700',
  [TaskStatus.Failed]: 'border-rose-300 bg-rose-50 text-rose-700'
};

export const SPEAKER_STATUS_TEXT: Record<SpeakerStatus, string> = {
  [SpeakerStatus.Ready]: '可用',
  [SpeakerStatus.Training]: '训练中',
  [SpeakerStatus.Disabled]: '已停用'
};

export const SPEAKER_STATUS_STYLES: Record<SpeakerStatus, string> = {
  [SpeakerStatus.Ready]: 'border-emerald-200 bg-emerald-50 text-emerald-700',
  [SpeakerStatus.Training]: 'border-amber-200 bg-amber-50 text-amber-700',
  [SpeakerStatus.Disabled]: 'border-slate-200 bg-slate-100 text-slate-600'
};
