export default {
  title: 'Text to Speech',
  description: 'Pick a model and optional speaker, enter text, configure model parameters and generate the target audio.',
  panels: {
    basic: 'Basic Parameters',
    recent: 'Recent Tasks',
    recentSubtitle: 'Shows the 5 most recent text-to-speech tasks, sourced from the unified history records',
    params: 'Generation Parameters',
    paramsSubtitle: 'Enter text and configure model-specific parameters to generate audio.'
  },
  speaker: {
    autoSelect: 'Auto select',
    autoSelectDesc: 'No speaker specified; the backend uses the default inference speaker of the current model or the model default configuration.',
    noDescription: 'This speaker has no notes yet.'
  },
  summary: {
    model: 'Current model: {model} {version}.',
    device: 'Current device: {device}.',
    dynamicReference: 'The current model provides reference audio and reference text via dynamic parameters.',
    speaker: 'Current speaker: {speaker}.',
    notSelected: 'Not selected',
    chars: '{chars} characters in {paragraphs} paragraphs.',
    format: 'Output format: {format}; export name: {name}.'
  },
  recent: {
    subtitle: 'Task {taskId} · {speaker} · {language}'
  },
  busy: {
    cancelling: 'Sending cancellation request, please wait',
    checkingDevice: 'Checking model environment, please wait',
    awaitingDeviceConfirm: 'Waiting for hardware environment switch confirmation',
    creating: 'Creating text-to-speech task, please wait',
    running: 'Task is running; the page keeps refreshing its status'
  },
  submit: {
    checking: 'Checking environment...',
    awaitingConfirm: 'Waiting for confirmation...',
    generating: 'Generating...',
    generate: 'Generate Audio'
  },
  form: {
    speaker: 'Speaker',
    speakerPlaceholder: 'Optional; auto-select when empty',
    language: 'Output Language',
    baseModel: 'Base Model',
    modelVersion: 'Model Version',
    deviceType: 'Device Type',
    format: 'Output Format',
    exportAudioName: 'Export Audio Name',
    exportNamePlaceholder: 'e.g. news_broadcast',
    inputText: 'Input Text',
    textPlaceholder: 'Enter the text to synthesize...',
    charStats: '{chars} characters, {paragraphs} paragraphs',
    modelParams: 'Model-Specific Parameters',
    summary: 'Generation Summary',
    cancel: 'Cancel Task',
    cancelling: 'Cancelling...',
    resetForm: 'Reset Form',
    refresh: 'Refresh Status',
    refreshing: 'Refreshing...',
    view: 'View'
  },
  result: {
    emptyText: 'No results yet. Enter text and click "Generate Audio"; results will appear here.',
    taskId: 'Task ID: {id}',
    createdAt: 'Generated at: {time}',
    exportName: 'Export name: {name}',
    copyTaskId: 'Copy Task ID',
    historyEmptyText: 'No history tasks yet. Generated audio will be added here automatically.'
  },
  dialog: {
    resetTitle: 'Reset Form',
    resetBody: 'The form will be reset to its default state, including all parameter settings. Generated result records will not be deleted. Continue?',
    confirmReset: 'Confirm Reset'
  },
  notice: {
    mismatchWarning: 'The target history task does not match this page type; cannot load its configuration.',
    parseFailed: 'Failed to parse the history task configuration.',
    loadFailed: 'Failed to load the text-to-speech history task configuration. Check the Rust backend logs',
    loadGenericFailed: 'Failed to load the history task configuration. Check whether the task record still exists',
    statusRefreshed: 'Text-to-speech task status refreshed.',
    refreshFailed: 'Failed to refresh text-to-speech history tasks. Check the Rust backend logs',
    refreshCurrentFailed: 'Failed to refresh the current task status. Check the Rust backend logs',
    submitting: 'Submitting the generation task.',
    submitted: 'Task submitted. Check its status and result in History.',
    generateFailed: 'Generation failed. Check the Rust backend logs',
    alreadyCancelling: 'A cancellation request has already been submitted for this task.',
    cancelRequested: 'Cancellation request sent. Task {taskId} will refresh once the backend stops it.',
    cancelFailed: 'Failed to cancel the task',
    formDefault: 'Form is at its default state.',
    formReset: 'Form has been reset.',
    noTaskId: 'No task ID to copy.',
    taskIdCopied: 'Task ID copied: {taskId}',
    copyFailed: 'Copy failed: clipboard permission is not available in this environment'
  }
};
