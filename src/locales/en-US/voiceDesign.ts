export default {
  title: 'Voice Design',
  description: 'Enter a voice description and target text to generate speech in the desired style.',
  panels: {
    basic: 'Basic Parameters',
    basicSubtitle: 'Supports qwen3_tts 1.7B and 0.6B versions; the device type is selected per task.',
    recent: 'Recent Tasks',
    recentSubtitle: 'Shows the 5 most recent voice design tasks, sourced from the unified history records',
    params: 'Generation Parameters',
    paramsSubtitle: 'Enter the voice design prompt and target text to generate the desired voice.'
  },
  summary: {
    model: 'Current model: {model} {version}',
    device: 'Current device: {device}',
    language: 'Output language: {language}',
    format: 'Output format: {format}',
    exportName: 'Export name: {name}'
  },
  recent: {
    subtitle: 'Task {taskId} · {language} · {fileName}'
  },
  busy: {
    cancelling: 'Sending cancellation request...',
    checkingDevice: 'Checking model environment...',
    awaitingDeviceConfirm: 'Waiting for hardware environment switch confirmation',
    creating: 'Creating voice design task...',
    running: 'Task is running; its status refreshes automatically.'
  },
  submit: {
    checking: 'Checking environment...',
    awaitingConfirm: 'Waiting for confirmation...',
    generating: 'Generating...',
    generate: 'Generate Audio'
  },
  form: {
    baseModel: 'Base Model',
    modelVersion: 'Model Version',
    deviceType: 'Device Type',
    language: 'Output Language',
    format: 'Output Format',
    exportAudioName: 'Export Audio Name',
    exportNamePlaceholder: 'e.g. voice_design_demo',
    prompt: 'Voice Design Prompt',
    promptPlaceholder: "e.g. A gentle, mature female voice with a steady pace, warm timbre and slight breathiness.",
    text: 'Target Text',
    textPlaceholder: 'Enter the target text to synthesize',
    charStats: '{chars} characters',
    modelParams: 'Model-Specific Parameters',
    emptyParamsTitle: 'This model has no model-specific parameters',
    emptyParamsDesc: 'No extra parameters to configure for this task; you can proceed to generate voice design directly.',
    summary: 'Generation Summary',
    cancel: 'Cancel Task',
    cancelling: 'Cancelling...',
    resetForm: 'Reset Form',
    refresh: 'Refresh Status',
    refreshing: 'Refreshing...',
    view: 'View'
  },
  result: {
    emptyText: 'No results yet. Provide the voice description and target text; results will appear here.',
    taskId: 'Task ID: {id}',
    createdAt: 'Generated at: {time}',
    exportName: 'Export name: {name}',
    prompt: 'Voice description: {text}',
    historyEmptyText: 'No history tasks yet. Generated audio will be added here automatically.'
  },
  notice: {
    statusRefreshed: 'Voice design task status refreshed.',
    refreshFailed: 'Failed to refresh voice design history tasks. Check the Rust backend logs.',
    refreshCurrentFailed: 'Failed to refresh the voice design task status. Check the backend logs.',
    notDesignTask: 'The selected history task is not a voice design task; cannot fill it back.',
    replayFilled: 'Parameters of task {taskId} filled in; you can generate again directly.',
    replayFailed: 'Failed to read the replay task',
    notDesignRecord: 'The selected record is not a voice design task.',
    loadDetailFailed: 'Failed to load the history task details. Check the backend logs.',
    submitting: 'Creating the voice design task.',
    created: 'Voice design task created, task ID {taskId}.',
    createFailed: 'Failed to create the voice design task',
    alreadyCancelling: 'A cancellation request has already been submitted for this task.',
    cancelRequested: 'Cancellation request sent. Task {taskId} will refresh once the backend stops it.',
    cancelFailed: 'Failed to cancel the task',
    formReset: 'Form has been reset.'
  }
};
