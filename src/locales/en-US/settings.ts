export default {
  title: 'Settings',
  description: 'Manage service connection, model resources and data log paths.',
  panelTitle: 'System Settings',
  uiLanguage: {
    label: 'UI Language'
  },
  tabs: {
    connection: 'Connection',
    model: 'Model Resources',
    cache: 'Cache'
  },
  connection: {
    serverUrl: 'Server URL',
    serverUrlPlaceholder: 'Enter the server address',
    apiToken: 'API Token',
    apiTokenPlaceholder: 'Enter the API token',
    save: 'Save Connection'
  },
  model: {
    hint: 'Manage local model resource directories and the unified Qwen attention implementation. Task execution devices are now selected on each task page and in the model installation UI.',
    modelDir: 'Model Directory',
    modelDirPlaceholder: 'Enter the model directory',
    attnLabel: 'Attention Implementation',
    save: 'Save Resources'
  },
  cache: {
    header: 'Cache Settings',
    hint: 'The data directory doubles as the training cache and the local business data root; the log directory is configured separately.',
    dataDir: 'Data Directory',
    dataDirPlaceholder: 'Enter the data directory path',
    logDir: 'Log Cache Path',
    logDirPlaceholder: 'Enter the log cache path',
    save: 'Save Cache Settings'
  },
  loading: {
    saving: 'Saving settings and migrating directories, please wait',
    reading: 'Reading current settings'
  },
  notice: {
    loaded: 'Current settings loaded.',
    loadFailed: 'Failed to read settings. Check the Rust backend and the config file',
    savedConnection: 'Connection settings saved.',
    savedModel: 'Model resource settings saved.',
    savedCache: 'Cache settings saved.',
    saveFailed: 'Failed to save settings. Check the input or the backend logs',
    migrated: 'Migrated {dirs}. Restart the app to switch to the new directory.',
    cleanupHint: 'Old directory contents are kept. Delete them manually once the new directory is confirmed working: {dirs}',
    cleanedHint: 'Old model directory contents were cleaned up automatically.'
  }
};
