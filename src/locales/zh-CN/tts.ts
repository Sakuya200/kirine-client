export default {
  title: '文本转语音',
  description: '选择模型，可选说话人，输入文本并配置模型参数，生成目标音频。',
  panels: {
    basic: '基础参数',
    recent: '最近任务',
    recentSubtitle: '展示最近 5 条文本转语音任务，数据来自统一历史记录',
    params: '生成参数',
    paramsSubtitle: '输入文本并配置模型特定参数后生成目标音频。'
  },
  speaker: {
    autoSelect: '自动选择',
    autoSelectDesc: '不指定说话人，后端会按当前模型使用默认推理说话人或模型内默认配置。',
    noDescription: '该说话人暂无备注。'
  },
  summary: {
    model: '当前模型为 {model} {version}。',
    device: '当前设备为 {device}。',
    dynamicReference: '当前模型通过动态参数提供参考音频与参考文本。',
    speaker: '当前说话人为 {speaker}。',
    notSelected: '未选择',
    chars: '当前字符数 {chars}，共 {paragraphs} 段。',
    format: '输出格式为 {format}，导出名称为 {name}。'
  },
  recent: {
    subtitle: '任务 {taskId} · {speaker} · {language}'
  },
  busy: {
    cancelling: '正在发送终止请求，请稍候',
    checkingDevice: '正在检查模型环境，请稍候',
    awaitingDeviceConfirm: '等待确认硬件环境切换',
    creating: '正在创建文本转语音任务，请稍候',
    running: '任务执行中，页面会持续刷新状态'
  },
  submit: {
    checking: '检查环境中...',
    awaitingConfirm: '等待确认...',
    generating: '生成中...',
    generate: '生成音频'
  },
  form: {
    speaker: '说话人',
    speakerPlaceholder: '可选，不指定时自动选择',
    language: '输出语言',
    baseModel: '基础模型',
    modelVersion: '模型版本',
    deviceType: '设备类型',
    format: '输出格式',
    exportAudioName: '导出音频名称',
    exportNamePlaceholder: '例如 news_broadcast',
    inputText: '输入文本',
    textPlaceholder: '请输入要合成的文本内容...',
    charStats: '字符数 {chars}，段落数 {paragraphs}',
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
    emptyText: '还没有生成结果。完成文本输入并点击“生成音频”后，结果会显示在这里。',
    taskId: '任务 ID：{id}',
    createdAt: '生成时间：{time}',
    exportName: '导出名称：{name}',
    copyTaskId: '复制任务ID',
    historyEmptyText: '还没有历史任务。生成音频后会自动加入这里。'
  },
  dialog: {
    resetTitle: '重置表单',
    resetBody: '表单将重置到默认状态，包括所有参数设置。已生成的结果记录不会被删除。确定继续吗？',
    confirmReset: '确认重置'
  },
  notice: {
    mismatchWarning: '目标历史任务与当前页面类型不匹配，无法载入配置。',
    parseFailed: '历史任务配置解析失败。',
    loadFailed: '载入文本转语音历史任务配置失败，请检查 Rust 后端日志',
    loadGenericFailed: '载入历史任务配置失败，请检查任务记录是否仍然存在',
    statusRefreshed: '文本转语音任务状态已刷新。',
    refreshFailed: '刷新文本转语音历史任务失败，请检查 Rust 后端日志',
    refreshCurrentFailed: '刷新当前任务状态失败，请检查 Rust 后端日志',
    submitting: '正在提交生成任务。',
    submitted: '任务已提交，可在历史记录中查看执行状态与结果。',
    generateFailed: '生成失败，请检查 Rust 后端日志',
    alreadyCancelling: '当前任务已经提交过终止请求。',
    cancelRequested: '已发送终止请求，任务 {taskId} 会在后端停止后刷新状态。',
    cancelFailed: '终止任务失败',
    formDefault: '表单已为默认状态。',
    formReset: '表单已重置。',
    noTaskId: '没有可复制的任务 ID。',
    taskIdCopied: '任务 ID 已复制：{taskId}',
    copyFailed: '复制失败，当前环境未开放剪贴板权限'
  }
};
