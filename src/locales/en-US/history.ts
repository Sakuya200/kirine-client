export default {
  title: 'History',
  description: 'View model training, text-to-speech, voice clone and voice design tasks in one place, with filtering, search, details and deletion.',
  panel: {
    title: 'Task List',
    subtitle: 'Shows model training, text-to-speech, voice clone and voice design tasks together.',
    refresh: 'Refresh List',
    refreshing: 'Refreshing...',
    searchPlaceholder: 'Search by task ID, title or speaker',
    loading: 'Loading history tasks...',
    empty: 'No history tasks match the current filters.'
  },
  table: {
    taskId: 'Task ID',
    taskName: 'Task Name',
    type: 'Type',
    speaker: 'Speaker',
    status: 'Status',
    duration: 'Duration',
    createTime: 'Created',
    actions: 'Actions',
    view: 'View',
    delete: 'Delete'
  },
  notice: {
    loadFailed: 'Failed to read history tasks. Check the local database or the Rust backend',
    updating: 'Updating history tasks, please wait',
    loading: 'Loading history task list',
    deleteFailed: 'Failed to delete the history task.',
    deleted: 'Task {title} deleted.',
    deleteError: 'Failed to delete the history task',
    alreadyCancelling: 'A cancellation request has already been submitted for this task.',
    cancelRequested: 'Cancellation request for task {taskId} sent.',
    cancelFailed: 'Failed to cancel the task'
  },
  deleteDialog: {
    title: 'Delete History Task',
    confirm: 'Task "{title}" will be deleted; associated detail records are logically deleted as well.',
    notFound: 'The history task to delete was not found.',
    confirmDelete: 'Confirm Delete',
    deleting: 'Deleting...'
  },
  taskDetail: {
    loadFailed: 'Failed to read the task details. Check whether the history record still exists',
    loading: 'Loading task details',
    title: 'Task Details',
    loadingText: 'Loading task details...',
    taskId: 'Task ID',
    taskStatus: 'Task Status',
    taskName: 'Task Name',
    speaker: 'Speaker',
    createTime: 'Created',
    modifyTime: 'Last Updated',
    params: 'Task Parameters',
    duration: 'Duration {duration}',
    logs: 'Task Logs',
    noLogs: 'No task logs yet.',
    cancelTask: 'Cancel Task',
    replay: 'Run Again',
    close: 'Close'
  }
};
