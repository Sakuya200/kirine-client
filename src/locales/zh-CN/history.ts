export default {
  title: '历史任务',
  description: '统一查看模型微调、文本转语音、声音克隆与音色设计任务，支持筛选、搜索、详情查看与删除。',
  panel: {
    title: '任务列表',
    subtitle: '统一展示模型微调、文本转语音、声音克隆与音色设计任务。',
    refresh: '刷新列表',
    refreshing: '刷新中...',
    searchPlaceholder: '按任务ID、标题或说话人搜索',
    loading: '正在加载历史任务...',
    empty: '当前筛选条件下没有匹配的历史任务。'
  },
  table: {
    taskId: '任务ID',
    taskName: '任务名称',
    type: '类型',
    speaker: '说话人',
    status: '状态',
    duration: '耗时',
    createTime: '创建时间',
    actions: '操作',
    view: '查看',
    delete: '删除'
  },
  notice: {
    loadFailed: '读取历史任务失败，请检查本地数据库或 Rust 后端',
    updating: '正在更新历史任务，请稍候',
    loading: '正在加载历史任务列表',
    deleteFailed: '删除历史任务失败。',
    deleted: '任务 {title} 已删除。',
    deleteError: '删除历史任务失败',
    alreadyCancelling: '当前任务已经提交过终止请求。',
    cancelRequested: '已发送任务 {taskId} 的终止请求。',
    cancelFailed: '终止任务失败'
  },
  deleteDialog: {
    title: '删除历史任务',
    confirm: '将删除任务“{title}”，该操作会同步逻辑删除关联详情记录。',
    notFound: '未找到要删除的历史任务。',
    confirmDelete: '确认删除',
    deleting: '删除中...'
  },
  taskDetail: {
    loadFailed: '读取任务详情失败，请检查历史记录是否仍然存在',
    loading: '任务详情加载中',
    title: '任务详情',
    loadingText: '正在加载任务详情...',
    taskId: '任务 ID',
    taskStatus: '任务状态',
    taskName: '任务名称',
    speaker: '说话人',
    createTime: '创建时间',
    modifyTime: '最近更新时间',
    params: '任务参数',
    duration: '耗时 {duration}',
    logs: '任务日志',
    noLogs: '暂无任务日志。',
    cancelTask: '终止任务',
    replay: '再次执行',
    close: '关闭'
  }
};
