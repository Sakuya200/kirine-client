export default {
  title: 'Model Management',
  description: 'View the base models supported by the system, their feature support and install status, and install or uninstall them.',
  panel: {
    title: 'Model List',
    subtitle: 'Base models available in the system with their supported features and install status',
    refresh: 'Refresh List',
    refreshing: 'Refreshing...',
    loading: 'Loading model list...',
    empty: 'No model information to show yet.'
  },
  busy: {
    mutating: 'Processing model install or uninstall, please wait',
    loading: 'Loading model list'
  },
  table: {
    model: 'Model',
    version: 'Version',
    features: 'Supported Features',
    dependencies: 'Dependencies',
    currentDevice: 'Current Device',
    status: 'Status',
    actions: 'Actions',
    devicePlaceholder: 'Select a device',
    deviceRequired: 'Select the current device first',
    reinstalling: 'Reinstalling...',
    installing: 'Installing...',
    reinstall: 'Reinstall',
    install: 'Install',
    uninstall: 'Uninstall'
  },
  deleteDialog: {
    title: 'Uninstall Model',
    confirm: 'The dedicated weight files of model "{name} {version}" will be uninstalled and its status set to not installed. Shared dependencies are kept.',
    confirmDelete: 'Confirm Uninstall',
    notFound: 'The model to uninstall was not found.',
    uninstalling: 'Uninstalling...'
  }
};
