export default {
  app: {
    name: 'Kirine Client'
  },
  save: 'Save',
  cancel: 'Cancel',
  delete: 'Delete',
  confirm: 'Confirm',
  loading: 'Loading...',
  saving: 'Saving...',
  retry: 'Retry',
  close: 'Close',
  colon: ': ',
  allStatuses: 'All statuses',
  allTaskTypes: 'All task types',
  taskTypeDetail: '{type} Details',
  taskStatus: {
    pending: 'Pending',
    running: 'Running',
    completed: 'Completed',
    cancelled: 'Cancelled',
    failed: 'Failed'
  },
  speakerStatus: {
    ready: 'Ready',
    training: 'Training',
    disabled: 'Disabled'
  },
  modelInstallStatus: {
    installed: 'Installed',
    notInstalled: 'Not installed',
    failed: 'Install failed'
  },
  historyTaskType: {
    modelTraining: 'Model Training',
    textToSpeech: 'Text to Speech',
    voiceClone: 'Voice Clone',
    voiceDesign: 'Voice Design',
    streamingSpeech: 'Streaming Speech'
  },
  modelTraining: {
    sampleType: {
      single: 'Single Sample',
      dataset: 'Dataset'
    }
  },
  textToSpeech: {
    format: {
      wav: 'WAV (lossless)',
      mp3: 'MP3 (compressed)',
      flac: 'FLAC (lossless compression)'
    }
  },
  ui: {
    selectPlaceholder: 'Select',
    perPage: '{size} / page',
    totalItems: '{total} items in total',
    range: '· items {start}-{end}',
    firstPage: 'First page',
    prevPage: 'Previous page',
    nextPage: 'Next page',
    lastPage: 'Last page',
    tooltipFallback: 'Show tooltip',
    defaultSpeaker: 'Default speaker',
    continue: 'Continue',
    enabled: 'Enabled',
    disabled: 'Disabled',
    selectAudioFile: 'Select Audio File',
    selectAudioButton: 'Select Audio',
    uploadText: 'Upload Text',
    selectTextFile: 'Select Text File',
    clear: 'Clear',
    audioFiles: 'Audio Files',
    textFiles: 'Text Files',
    noAudioSelected: 'No audio file selected yet',
    noTextSelected: 'No text file selected yet',
    emptyParamsTitle: 'This model has no model-specific parameters',
    emptyParamsDesc: 'No extra parameters to configure for this task; you can proceed directly.'
  },
  audio: {
    downloadLabel: 'Download Audio',
    pendingMessage: 'The task is still running; the audio result will appear once the status becomes "Completed".',
    failedMessage: 'The task failed and produced no audio. Check the "Task Logs" in the history task details for the failure reason.',
    cancelledMessage: 'The task was cancelled; no audio result.',
    ended: 'Audio playback finished.',
    playFailed: 'Audio playback failed. Check whether the audio file is still readable.',
    playBlocked: 'Audio playback failed; the current environment may have blocked playback',
    pause: 'Pause',
    play: 'Play Audio',
    noResult: 'No playable audio result yet.',
    notCompleted: 'The task is not completed yet; audio can be played once it finishes.',
    paused: 'Audio playback paused.',
    noDownload: 'No downloadable audio result yet.',
    notCompletedDownload: 'The task is not completed yet; wait for a status update before downloading.',
    downloadCancelled: 'Download cancelled.',
    saved: 'Audio saved.',
    downloadFailed: 'Download failed. Check the system save dialog permissions and the output file status',
    decodeFailed: 'Audio playback failed. Check whether the audio data can be decoded.',
    noPlayableIndex: 'No playable audio index yet.',
    noDownloadIndex: 'No downloadable audio index yet.',
    loadingAudio: 'Loading audio',
    waitingData: 'Waiting for data'
  },
  resultCard: {
    title: 'Generation Result',
    subtitle: 'Shows the latest task result and output file information',
    loadingDetail: 'Loading...',
    viewDetail: 'View Details',
    download: 'Download'
  },
  recentList: {
    actionLabel: 'View'
  },
  deviceGuard: {
    title: 'Hardware Environment Confirmation',
    message: 'The selected hardware type differs from what the model environment currently supports. If you continue, the current environment will be updated automatically. This will not affect normal task execution but will significantly increase the execution time of this task. Continue?',
    checkFailed: 'Failed to query the current hardware type of the model environment',
    model: 'Model: {model} {version}',
    currentEnvType: 'Current environment type: {device}',
    selectedType: 'Selected type: {device}'
  },
  store: {
    models: {
      loadFailed: 'Failed to load the model list',
      installed: 'Model {name} {version} installed.',
      installFailed: 'Failed to install the model',
      uninstalled: 'Model {name} {version} uninstalled.',
      uninstallFailed: 'Failed to uninstall the model',
      reinstallAfterUninstallFailed: 'Model uninstall failed; cannot reinstall. Please retry the uninstall.',
      reinstallAfterUninstall: 'The model was uninstalled but the reinstall failed. Please retry the install.',
      setCurrentDeviceFailed: 'Failed to set the current device'
    },
    speakers: {
      loadFailed: 'Failed to load the speaker list',
      created: 'Speaker "{name}" created.',
      createFailed: 'Failed to create the speaker',
      updated: 'Speaker "{name}" updated.',
      updateFailed: 'Failed to save the speaker information',
      imported: 'Speaker "{name}" imported.',
      importFailed: 'Failed to import the speaker',
      deleteFailed: 'Failed to delete the speaker.',
      deleted: 'Speaker "{name}" deleted.'
    },
    uiConfig: {
      loadFailed: 'Failed to load the UI parameter configuration'
    },
    pagination: {
      loadFailed: 'Failed to load the list'
    }
  },
  languageName: {
    chinese: 'Chinese',
    english: 'English',
    japanese: 'Japanese',
    korean: 'Korean'
  },
  notFound: {
    title: 'Page Not Found',
    description: 'Return to a valid page from the navigation on the left.',
    backToTraining: 'Back to Model Training'
  }
};
