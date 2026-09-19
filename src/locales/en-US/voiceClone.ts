export default {
  title: 'Voice Clone',
  description: 'Generate new speech audio from a reference audio, reference text and target text.',
  panels: {
    basic: 'Basic Parameters',
    basicSubtitle: 'The reference audio and reference text must match exactly; the hardware type is selected per task.',
    recent: 'Recent Tasks',
    recentSubtitle: 'Shows the 5 most recent voice clone tasks, sourced from the unified history records',
    params: 'Generation Parameters',
    paramsSubtitle: 'Enter the target text and configure model-specific parameters to generate new speech audio.'
  },
  summary: {
    model: 'Current model: {model} {version}.',
    device: 'Current device: {device}.',
    language: 'Current language: {language}.',
    format: 'Output format: {format}.',
    exportName: 'Export name: {name}.'
  },
  recent: {
    subtitle: 'Task {taskId} · {language} · {fileName}'
  },
  busy: {
    cancelling: 'Sending cancellation request, please wait',
    checkingDevice: 'Checking model environment, please wait',
    awaitingDeviceConfirm: 'Waiting for hardware environment switch confirmation',
    creating: 'Creating voice clone task, please wait',
    running: 'Task is running; the page keeps refreshing its status'
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
    exportNamePlaceholder: 'e.g. clone_demo',
    refAudio: 'Reference Audio',
    selectAudio: 'Select Audio',
    noRefAudio: 'No reference audio selected yet',
    refText: 'Reference Text (optional)',
    refTextPlaceholder: 'Optional; the text actually spoken in the reference audio',
    text: 'Target Text',
    textPlaceholder: 'Enter the target text to synthesize into new audio',
    charStats: '{refChars} characters of reference text, {chars} characters of target text',
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
    emptyText: 'No results yet. Provide the reference audio and text; results will appear here.',
    taskId: 'Task ID: {id}',
    createdAt: 'Generated at: {time}',
    exportName: 'Export name: {name}',
    refAudio: 'Reference audio: {name}',
    refText: 'Reference text: {text}',
    historyEmptyText: 'No history tasks yet. Generated audio will be added here automatically.'
  },
  dialog: {
    selectRefAudio: 'Select Reference Audio',
    audioFiles: 'Audio Files'
  },
  notice: {
    mismatchWarning: 'The target history task does not match this page type; cannot load its configuration.',
    parseFailed: 'Failed to parse the history task configuration.',
    loadFailed: 'Failed to load the voice clone history task configuration. Check the backend logs',
    loadGenericFailed: 'Failed to load the history task configuration. Check whether the task record still exists',
    replayLoaded: 'Configuration of history task {historyId} loaded. Please create a new task.',
    pickerFailed: 'Failed to open the file picker',
    statusRefreshed: 'Voice clone task status refreshed.',
    refreshFailed: 'Failed to refresh voice clone history tasks. Check the Rust backend logs',
    refreshCurrentFailed: 'Failed to refresh the voice clone task status. Check the backend logs',
    submitting: 'Creating the voice clone task.',
    created: 'Voice clone task created, task ID {taskId}.',
    createLogFailed: 'Failed to create the voice clone task: ',
    createFailed: 'Failed to create the voice clone task',
    alreadyCancelling: 'A cancellation request has already been submitted for this task.',
    cancelRequested: 'Cancellation request sent. Task {taskId} will refresh once the backend stops it.',
    cancelFailed: 'Failed to cancel the task',
    formReset: 'Form has been reset.'
  }
};
