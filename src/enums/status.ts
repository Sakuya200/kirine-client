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

export const STATUS_TEXT_KEY: Record<TaskStatus, string> = {
  [TaskStatus.Pending]: 'common.taskStatus.pending',
  [TaskStatus.Running]: 'common.taskStatus.running',
  [TaskStatus.Completed]: 'common.taskStatus.completed',
  [TaskStatus.Cancelled]: 'common.taskStatus.cancelled',
  [TaskStatus.Failed]: 'common.taskStatus.failed'
};

export const STATUS_STYLES: Record<TaskStatus, string> = {
  [TaskStatus.Pending]: 'border-brand-200 bg-brand-50 text-brand-700',
  [TaskStatus.Running]: 'border-amber-300 bg-amber-50 text-amber-700',
  [TaskStatus.Completed]: 'border-emerald-300 bg-emerald-50 text-emerald-700',
  [TaskStatus.Cancelled]: 'border-slate-300 bg-slate-100 text-slate-700',
  [TaskStatus.Failed]: 'border-rose-300 bg-rose-50 text-rose-700'
};

export enum ModelInstallStatus {
  Installed = 'installed',
  NotInstalled = 'not-installed',
  Failed = 'failed'
}

export const SPEAKER_STATUS_TEXT_KEY: Record<SpeakerStatus, string> = {
  [SpeakerStatus.Ready]: 'common.speakerStatus.ready',
  [SpeakerStatus.Training]: 'common.speakerStatus.training',
  [SpeakerStatus.Disabled]: 'common.speakerStatus.disabled'
};

export const SPEAKER_STATUS_STYLES: Record<SpeakerStatus, string> = {
  [SpeakerStatus.Ready]: 'border-emerald-200 bg-emerald-50 text-emerald-700',
  [SpeakerStatus.Training]: 'border-amber-200 bg-amber-50 text-amber-700',
  [SpeakerStatus.Disabled]: 'border-slate-200 bg-slate-100 text-slate-600'
};

export const MODEL_INSTALL_STATUS_TEXT_KEY: Record<ModelInstallStatus, string> = {
  [ModelInstallStatus.Installed]: 'common.modelInstallStatus.installed',
  [ModelInstallStatus.NotInstalled]: 'common.modelInstallStatus.notInstalled',
  [ModelInstallStatus.Failed]: 'common.modelInstallStatus.failed'
};

export const MODEL_INSTALL_STATUS_STYLES: Record<ModelInstallStatus, string> = {
  [ModelInstallStatus.Installed]: 'border-emerald-200 bg-emerald-50 text-emerald-700',
  [ModelInstallStatus.NotInstalled]: 'border-stone-200 bg-stone-100 text-stone-600',
  [ModelInstallStatus.Failed]: 'border-rose-300 bg-rose-50 text-rose-700'
};
