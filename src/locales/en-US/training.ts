export default {
  title: 'Model Training',
  description: 'Upload fine-tuning samples and annotations, configure parameters and create a local speaker model usable for text-to-speech.',
  panels: {
    import: 'Fine-tuning Data Import',
    importSubtitle: 'Supports per-sample and dataset import; both require aligned audio and text',
    checklist: 'Preparation Checklist',
    checklistSubtitle: 'Make sure the data quality is good before starting fine-tuning',
    taskInfo: 'Fine-tuning Task Info',
    taskInfoSubtitle: 'Shows key information of the focused task; open the full details directly.',
    recent: 'Recent Tasks',
    recentSubtitle: 'Shows the 5 most recent model training tasks, sourced from the unified history records',
    params: 'Fine-tuning Parameters',
    paramsSubtitle: 'Model fine-tuning parameter configuration'
  },
  checklist: {
    item1: 'Prepare at least 1 audio sample with its transcript as fine-tuning input; 24kHz+ quality recommended.',
    item2: 'Keep the sample language consistent; switching languages mid-way causes style drift.',
    item3: 'Trim silent segments before uploading; keep each sample within 15 seconds.'
  },
  recent: {
    subtitle: 'Task {taskId} · {model}'
  },
  info: {
    notFilled: 'Not filled',
    taskMeta: 'Task ID {taskId} · {createTime}',
    speakerName: 'Speaker name {name}.',
    speakerDescription: 'Speaker description {description}.',
    model: 'Model {model} {version}.',
    sampleCount: '{count} samples.',
    empty: 'No fine-tuning task to show yet. Create a task or pick one from Recent Tasks to see its key information here.',
    viewDetail: 'View Details'
  },
  busy: {
    checkingDevice: 'Checking model environment, please wait',
    awaitingDeviceConfirm: 'Waiting for hardware environment switch confirmation',
    creating: 'Creating model training task, please wait',
    cancelling: 'Requesting cancellation of the model training task, please wait',
    running: 'Task is running; the page keeps refreshing its status'
  },
  submit: {
    checking: 'Checking environment...',
    awaitingConfirm: 'Waiting for confirmation...',
    creating: 'Creating...',
    start: 'Start Fine-tuning'
  },
  single: {
    badge: 'Single Sample',
    heading: 'One-to-one Upload',
    desc: 'Provide one audio clip and its transcript; imported as single-sample fine-tuning data.',
    audio: 'Audio File',
    selectAudio: 'Select Local Audio',
    noAudio: 'No audio file selected yet',
    text: 'Transcript',
    textPlaceholder: 'Enter the transcript that matches the audio exactly...',
    add: 'Add to Import List',
    added: 'Single sample added to the import list; you can add more data.',
    detail: 'Audio file · {path}'
  },
  dataset: {
    badge: 'Dataset',
    heading: 'Batch Upload',
    desc: 'Provide an audio archive and an annotation file; imported as batch fine-tuning data.',
    archive: 'Audio Archive',
    selectZip: 'Select ZIP Archive',
    noZip: 'No ZIP archive selected yet',
    annotation: 'Annotation File',
    downloadTemplate: 'Download Template',
    selectAnnotation: 'Select Annotation File',
    noAnnotation: 'No annotation file selected yet',
    formatHint: 'Supports {jsonl}, {xlsx} and {xls}',
    add: 'Add to Import List',
    added: 'Dataset added to the import list; it will be processed as a dataset during training.',
    detail: 'ZIP archive + annotation file · {path}'
  },
  list: {
    heading: 'Import List',
    summary: '{total} samples imported.',
    remove: 'Remove',
    empty: 'No samples imported yet. Both single samples and datasets can be added to the fine-tuning list.'
  },
  dialog: {
    selectAudio: 'Select Audio File',
    selectZip: 'Select ZIP Archive',
    selectAnnotation: 'Select Annotation File',
    audioFiles: 'Audio Files',
    zipFiles: 'ZIP Archive',
    annotationFiles: 'Annotation Files'
  },
  form: {
    speakerName: 'Speaker Name',
    speakerNamePlaceholder: 'Enter the speaker name',
    baseModel: 'Base Model',
    modelVersion: 'Model Version',
    deviceType: 'Device Type',
    language: 'Language',
    speakerDescription: 'Speaker Description',
    speakerDescriptionPlaceholder: 'Enter the speaker description to identify the model in the speaker list later',
    modelParams: 'Model-Specific Fine-tuning Parameters',
    modelParamsHint: 'Available fine-tuning parameters differ between base models; refer to each model official documentation.',
    summary: 'Fine-tuning Summary',
    summaryData: 'Using {total} imported samples, language {language}.',
    notSelected: 'Not selected',
    summaryModel: 'Base model {model} {version}.',
    summaryDevice: 'Device type {device}.',
    summaryDescription: 'Speaker description {description}.',
    summaryBatch: 'Adjust the batch size to your VRAM; start with 4-8 for small samples.',
    summaryGradAccum: 'Current gradient accumulation: {steps}.',
    cancel: 'Cancel Fine-tuning',
    cancelling: 'Cancelling...',
    resetForm: 'Reset Form',
    refresh: 'Refresh Status',
    refreshing: 'Refreshing...',
    view: 'View'
  },
  notice: {
    pickerFailed: 'Failed to open the file picker',
    formReset: 'Training form has been reset.',
    mismatchWarning: 'The target history task does not match this page type; cannot load its configuration.',
    loadFailed: 'Failed to load the model training history task configuration. Check the Rust backend logs',
    loadGenericFailed: 'Failed to load the history task configuration. Check whether the task record still exists',
    statusRefreshed: 'Model training task status refreshed.',
    refreshFailed: 'Failed to refresh model training history tasks. Check the Rust backend logs',
    refreshCurrentFailed: 'Failed to refresh the model training task status. Check the Rust backend logs',
    alreadyCancelling: 'A cancellation request has already been submitted for this task.',
    cancelRequested: 'Cancellation request sent. Task {taskId} will refresh once the backend stops it.',
    cancelFailed: 'Failed to cancel the task',
    submitting: 'Creating the model training task.',
    created: 'Model training task created: {speaker}, task ID {taskId}, base model {model} {version}, {count} samples.',
    createFailed: 'Failed to create the model training task'
  }
};
