export default {
  title: '声音克隆',
  description: '使用参考音频、参考台词和目标文本生成新的语音音频。',
  panels: {
    basic: '基础参数',
    basicSubtitle: '参考音频与参考台词必须严格对应，硬件类型按当前任务单独选择。',
    recent: '最近任务',
    recentSubtitle: '展示最近 5 条声音克隆任务，数据来自统一历史记录',
    params: '生成参数',
    paramsSubtitle: '输入目标台词并配置模型特定参数后生成新的语音音频。'
  },
  summary: {
    model: '当前模型为 {model} {version}。',
    device: '当前设备为 {device}。',
    language: '当前语言为 {language}。',
    format: '输出格式为 {format}。',
    exportName: '导出名称为 {name}。'
  },
  recent: {
    subtitle: '任务 {taskId} · {language} · {fileName}'
  },
  busy: {
    cancelling: '正在发送终止请求，请稍候',
    checkingDevice: '正在检查模型环境，请稍候',
    awaitingDeviceConfirm: '等待确认硬件环境切换',
    creating: '正在创建声音克隆任务，请稍候',
    running: '任务执行中，页面会持续刷新状态'
  },
  submit: {
    checking: '检查环境中...',
    awaitingConfirm: '等待确认...',
    generating: '生成中...',
    generate: '生成音频'
  },
  form: {
    baseModel: '基础模型',
    modelVersion: '模型版本',
    deviceType: '设备类型',
    language: '输出语言',
    format: '输出格式',
    exportAudioName: '导出音频名称',
    exportNamePlaceholder: '例如 clone_demo',
    refAudio: '参考音频',
    selectAudio: '选择音频',
    noRefAudio: '尚未选择参考音频',
    refText: '参考台词（可选）',
    refTextPlaceholder: '可选，填写参考音频中实际说出的文本',
    text: '目标台词',
    textPlaceholder: '填写要合成为新音频的目标文本',
    charStats: '参考台词 {refChars} 字，目标台词 {chars} 字',
    modelParams: '模型特定参数',
    summary: '生成摘要',
    cancel: '终止任务',
    cancelling: '终止中...',
    resetForm: '重置表单',
    refresh: '刷新状态',
    refreshing: '刷新中...',
    view: '查看'
  },
  result: {
    emptyText: '还没有生成结果。完成参考音频和文本输入后，结果会显示在这里。',
    taskId: '任务 ID：{id}',
    createdAt: '生成时间：{time}',
    exportName: '导出名称：{name}',
    refAudio: '参考音频：{name}',
    refText: '参考台词：{text}',
    historyEmptyText: '还没有历史任务。生成音频后会自动加入这里。'
  },
  dialog: {
    selectRefAudio: '选择参考音频',
    audioFiles: '音频文件'
  },
  notice: {
    mismatchWarning: '目标历史任务与当前页面类型不匹配，无法载入配置。',
    parseFailed: '历史任务配置解析失败。',
    loadFailed: '载入声音克隆历史任务配置失败，请检查后端日志',
    loadGenericFailed: '载入历史任务配置失败，请检查任务记录是否仍然存在',
    replayLoaded: '已载入历史任务 {historyId} 的配置，请重新创建新任务。',
    pickerFailed: '打开文件选择器失败',
    statusRefreshed: '声音克隆任务状态已刷新。',
    refreshFailed: '刷新声音克隆历史任务失败，请检查 Rust 后端日志',
    refreshCurrentFailed: '刷新声音克隆任务状态失败，请检查后端日志',
    submitting: '正在创建声音克隆任务。',
    created: '声音克隆任务已创建，任务 ID {taskId}。',
    createLogFailed: '创建声音克隆任务失败：',
    createFailed: '声音克隆任务创建失败',
    alreadyCancelling: '当前任务已经提交过终止请求。',
    cancelRequested: '已发送终止请求，任务 {taskId} 会在后端停止后刷新状态。',
    cancelFailed: '终止任务失败',
    formReset: '表单已重置。'
  }
};
