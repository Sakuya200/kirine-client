export default {
  title: 'Speaker Management',
  description: 'View, filter, edit and delete trained speakers.',
  stats: {
    total: 'Total speakers',
    readyModels: 'Ready models',
    training: 'Training',
    totalSamples: 'Total samples'
  },
  panel: {
    title: 'Speaker List',
    subtitle: 'Search, filter by status, view details, edit and delete',
    importModel: 'Import Model',
    refresh: 'Refresh List',
    refreshing: 'Refreshing...',
    searchPlaceholder: 'Search by name or notes',
    samples: '{count} samples',
    createdAt: 'Created {time} · last updated {modifyTime}',
    viewDetail: 'View Details',
    edit: 'Edit',
    delete: 'Delete',
    loading: 'Loading speaker list...',
    empty: 'No speakers match the current filters.'
  },
  detail: {
    title: 'Speaker Details',
    name: 'Name: ',
    model: 'Model: ',
    samples: 'Samples: ',
    status: 'Status: ',
    createTime: 'Created: ',
    modifyTime: 'Updated: ',
    description: 'Notes: ',
    close: 'Close'
  },
  editDialog: {
    title: 'Edit Speaker',
    name: 'Name',
    namePlaceholder: 'Enter the speaker name',
    description: 'Notes',
    descriptionPlaceholder: 'Enter usage notes, applicable scenarios or management notes',
    cancel: 'Cancel',
    save: 'Save Changes'
  },
  importDialog: {
    title: 'Import External Model',
    modelType: 'Model Type',
    modelVersion: 'Model Version',
    modelDir: 'Model Directory',
    dirPlaceholder: 'Select the directory of the downloaded model',
    selectDir: 'Select Directory',
    dirPickerTitle: 'Select the downloaded model directory',
    speakerName: 'Speaker Name',
    speakerNamePlaceholder: 'Enter the speaker name',
    speakerDescription: 'Speaker Description',
    speakerDescriptionPlaceholder: 'Enter the speaker description or use case',
    cancel: 'Cancel',
    confirm: 'Confirm Import',
    importing: 'Importing...'
  },
  deleteDialog: {
    title: 'Delete Speaker',
    confirm: 'Speaker "{name}" will be deleted. This performs a logical delete and removes it from database query results.',
    notFound: 'The speaker to delete was not found.',
    cancel: 'Cancel',
    confirmDelete: 'Confirm Delete',
    deleting: 'Deleting...'
  }
};
