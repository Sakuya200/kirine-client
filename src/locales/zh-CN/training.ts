export default {
  title: '模型微调',
  description: '上传微调样本与标注内容，配置参数并创建可用于文本转语音的本地说话人模型。',
  panels: {
    import: '微调数据导入',
    importSubtitle: '支持逐条导入和按数据集导入，两种方式都需要音频与文本对齐',
    checklist: '准备清单',
    checklistSubtitle: '开始微调前，先确保数据质量',
    taskInfo: '微调任务信息',
    taskInfoSubtitle: '展示当前聚焦任务的关键信息，可直接查看完整详情。',
    recent: '最近任务',
    recentSubtitle: '展示最近 5 条模型微调任务，数据来自统一历史记录',
    params: '微调参数',
    paramsSubtitle: '模型微调参数配置'
  },
  checklist: {
    item1: '准备最少 1 条音频样本与对应文本稿，作为微调数据输入，建议 24kHz 以上质量。',
    item2: '确保样本语言一致，避免中途切换语言导致风格漂移。',
    item3: '上传前先裁剪静音段，控制单条样本长度在 15 秒以内。'
  },
  recent: {
    subtitle: '任务 {taskId} · {model}'
  },
  info: {
    notFilled: '未填写',
    taskMeta: '任务 ID {taskId} · {createTime}',
    speakerName: '说话人名称 {name}。',
    speakerDescription: '说话人描述 {description}。',
    model: '使用模型 {model} {version}。',
    sampleCount: '样本数 {count} 项。',
    empty: '还没有可展示的微调任务。创建任务后，或从最近任务中选中一项后，会在这里显示关键信息。',
    viewDetail: '查看详情'
  },
  busy: {
    checkingDevice: '正在检查模型环境，请稍候',
    awaitingDeviceConfirm: '等待确认硬件环境切换',
    creating: '正在创建模型微调任务，请稍候',
    cancelling: '正在请求终止模型微调任务，请稍候',
    running: '任务执行中，页面会持续刷新状态'
  },
  submit: {
    checking: '检查环境中...',
    awaitingConfirm: '等待确认...',
    creating: '创建中...',
    start: '开始微调'
  },
  single: {
    badge: '单样本',
    heading: '一对一上传',
    desc: '提供一段音频和对应台词，导入后记为单样本微调数据。',
    audio: '音频文件',
    selectAudio: '选择本地音频',
    noAudio: '尚未选择音频文件',
    text: '台词文本',
    textPlaceholder: '请输入与音频严格对应的台词文本...',
    add: '加入导入列表',
    added: '已将单样本加入导入列表，可继续添加更多数据。',
    detail: '音频文件 · {path}'
  },
  dataset: {
    badge: '样本集',
    heading: '批量上传',
    desc: '提供音频压缩包与数据标注文件，导入后记为批量微调数据。',
    archive: '音频压缩包',
    selectZip: '选择 ZIP 压缩包',
    noZip: '尚未选择 ZIP 压缩包',
    annotation: '数据标注文件',
    downloadTemplate: '下载模板',
    selectAnnotation: '选择数据标注文件',
    noAnnotation: '尚未选择标注文件',
    formatHint: '支持 {jsonl}、{xlsx} 和 {xls}',
    add: '加入导入列表',
    added: '已将样本集加入导入列表，训练时会按数据集方式处理。',
    detail: 'ZIP 压缩包 + 标注文件 · {path}'
  },
  list: {
    heading: '导入列表',
    summary: '已导入 {total} 项样本。',
    remove: '移除',
    empty: '还没有导入任何样本。单样本和样本集都可以加入微调列表。'
  },
  dialog: {
    selectAudio: '选择音频文件',
    selectZip: '选择 ZIP 压缩包',
    selectAnnotation: '选择数据标注文件',
    audioFiles: '音频文件',
    zipFiles: 'ZIP 压缩包',
    annotationFiles: '数据标注文件'
  },
  form: {
    speakerName: '说话人名称',
    speakerNamePlaceholder: '请输入说话人名称',
    baseModel: '基础模型',
    modelVersion: '模型版本',
    deviceType: '设备类型',
    language: '语种',
    speakerDescription: '说话人描述',
    speakerDescriptionPlaceholder: '请输入说话人描述，用于后续在说话人列表中识别模型',
    modelParams: '模型特定微调参数',
    modelParamsHint: '当选择不同的基础模型时，可配置的微调参数会有所不同，请参考不同模型的官方文档。',
    summary: '微调摘要',
    summaryData: '当前将使用 {total} 项导入数据，语言 {language}。',
    notSelected: '未选择',
    summaryModel: '基础模型 {model} {version}。',
    summaryDevice: '设备类型 {device}。',
    summaryDescription: '说话人描述 {description}。',
    summaryBatch: '建议批次大小根据显存调整，样本较少时可先从 4 到 8 开始。',
    summaryGradAccum: '当前梯度累积 {steps}。',
    cancel: '终止微调',
    cancelling: '终止中...',
    resetForm: '重置表单',
    refresh: '刷新状态',
    refreshing: '刷新中...',
    view: '查看'
  },
  notice: {
    pickerFailed: '打开文件选择器失败',
    formReset: '训练表单已重置。',
    mismatchWarning: '目标历史任务与当前页面类型不匹配，无法载入配置。',
    loadFailed: '载入模型微调历史任务配置失败，请检查 Rust 后端日志',
    loadGenericFailed: '载入历史任务配置失败，请检查任务记录是否仍然存在',
    statusRefreshed: '模型微调任务状态已刷新。',
    refreshFailed: '刷新模型微调历史任务失败，请检查 Rust 后端日志',
    refreshCurrentFailed: '刷新模型微调任务状态失败，请检查 Rust 后端日志',
    alreadyCancelling: '当前任务已经提交过终止请求。',
    cancelRequested: '已发送终止请求，任务 {taskId} 会在后端停止后刷新状态。',
    cancelFailed: '终止任务失败',
    submitting: '正在创建模型微调任务。',
    created: '模型微调任务已创建：{speaker}，任务 ID {taskId}，基础模型 {model} {version}，共 {count} 项样本。',
    createFailed: '模型微调任务创建失败'
  },
  template: {
    jsonlSaved: 'JSONL 模板已保存。',
    xlsxSaved: 'Excel 模板已保存。',
    saveFailed: '保存模板文件失败',
    jsonlDesc: '每行一个 JSON 对象，适合脚本批量处理或版本管理。',
    jsonlHint: "{'{'}\"audio\": \"speaker_001.wav\", \"text\": \"这里填写台词\"{'}'}",
    xlsxDesc: '首列填写文件名，第二列填写台词，适合直接用 Excel 编辑。',
    xlsxHint: '第一列 文件名 / 第二列 台词',
    dialogTitle: '下载数据标注模板',
    downloading: '下载中...',
    downloadXlsx: '下载Excel模板',
    downloadNamed: '下载 {title} 模板',
    oggHint: '如果压缩包中的音频是 OGG，后端会在训练前自动转成 WAV 后再交给模型处理。',
    dialogIntro: '选择一种模板格式下载。JSONL 与 Excel 模板都使用相同的数据结构，Excel 导入时会读取第一列文件名与第二列台词。',
    xlsxSupport: 'Excel 导入支持 .xlsx 与 .xls。',
    close: '关闭'
  }
};
