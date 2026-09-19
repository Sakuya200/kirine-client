export default {
  title: '音色设计',
  description: '输入音色描述与目标台词，生成符合指定风格的语音音频。',
  panels: {
    basic: '基础参数',
    basicSubtitle: '支持 qwen3_tts 的 1.7B 与 0.6B 版本，设备类型按任务单独选择。',
    recent: '最近任务',
    recentSubtitle: '展示最近 5 条音色设计任务，数据来自统一历史记录',
    params: '生成参数',
    paramsSubtitle: '输入音色描述 Prompt 与目标台词，生成符合需求的人声。'
  },
  summary: {
    model: '当前模型：{model} {version}',
    device: '当前设备：{device}',
    language: '输出语言：{language}',
    format: '输出格式：{format}',
    exportName: '导出名称：{name}'
  },
  recent: {
    subtitle: '任务 {taskId} · {language} · {fileName}'
  },
  busy: {
    cancelling: '正在发送终止请求...',
    checkingDevice: '正在检查模型环境...',
    awaitingDeviceConfirm: '等待确认硬件环境切换',
    creating: '正在创建音色设计任务...',
    running: '任务执行中，状态会自动刷新。'
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
    exportNamePlaceholder: '例如 voice_design_demo',
    prompt: '音色描述 Prompt',
    promptPlaceholder: '例如：体现温柔成熟的女声，语速平稳，音色偏暖，带轻微气声。',
    text: '目标台词',
    textPlaceholder: '填写需要合成的目标文本',
    charStats: '当前字符数 {chars}',
    modelParams: '模型特定参数',
    emptyParamsTitle: '当前模型没有可配置的特定参数',
    emptyParamsDesc: '这个任务下没有额外参数需要配置，可以直接继续生成音色设计。',
    summary: '生成摘要',
    cancel: '终止任务',
    cancelling: '终止中...',
    resetForm: '重置表单',
    refresh: '刷新状态',
    refreshing: '刷新中...',
    view: '查看'
  },
  result: {
    emptyText: '还没有生成结果。完成音色描述和目标文本输入后，结果会显示在这里。',
    taskId: '任务 ID：{id}',
    createdAt: '生成时间：{time}',
    exportName: '导出名称：{name}',
    prompt: '音色描述：{text}',
    historyEmptyText: '还没有历史任务。生成音频后会自动加入这里。'
  },
  notice: {
    statusRefreshed: '音色设计任务状态已刷新。',
    refreshFailed: '刷新音色设计历史任务失败，请检查 Rust 后端日志。',
    refreshCurrentFailed: '刷新音色设计任务状态失败，请检查后端日志。',
    notDesignTask: '当前历史任务不是音色设计任务，无法回填。',
    replayFilled: '已回填任务 {taskId} 的参数，可直接再次生成。',
    replayFailed: '读取回放任务失败',
    notDesignRecord: '当前记录不是音色设计任务。',
    loadDetailFailed: '加载历史任务详情失败，请检查后端日志。',
    submitting: '正在创建音色设计任务。',
    created: '音色设计任务已创建，任务 ID {taskId}。',
    createFailed: '音色设计任务创建失败',
    alreadyCancelling: '当前任务已经提交过终止请求。',
    cancelRequested: '已发送终止请求，任务 {taskId} 会在后端停止后刷新状态。',
    cancelFailed: '终止任务失败',
    formReset: '表单已重置。'
  }
};
