export default {
  model: {
    deviceUnsupported: 'Model {model} does not support device {device}. Please switch to: {supported}',
    notInstalled: 'Please install the model on the Model Management page first ({model})'
  },
  task: {
    handleReadFailed: 'Failed to read the running task handle',
    terminateSignalFailed: 'Failed to send the task termination signal'
  },
  settings: {
    readStateFailed: 'Failed to read the settings state',
    writeStateFailed: 'Failed to write the settings state'
  },
  validation: {
    speakerNameRequired: 'Speaker name is required',
    speakerDescriptionRequired: 'Speaker description is required',
    baseModelTypeRequired: 'Base model type is required',
    modelVersionRequired: 'Model version is required',
    refAudioRequired: 'Reference audio is required',
    textRequired: 'Target text is required',
    promptRequired: 'Voice description is required'
  }
};
