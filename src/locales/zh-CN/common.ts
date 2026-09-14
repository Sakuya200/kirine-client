export default {
  app: {
    name: 'Kirine Client'
  },
  save: '保存',
  cancel: '取消',
  delete: '删除',
  confirm: '确认',
  loading: '加载中...',
  saving: '保存中...',
  retry: '重试',
  close: '关闭',
  colon: '：',
  allStatuses: '全部状态',
  allTaskTypes: '全部任务类型',
  taskTypeDetail: '{type}详情',
  taskStatus: {
    pending: '待执行',
    running: '执行中',
    completed: '已完成',
    cancelled: '已终止',
    failed: '失败'
  },
  speakerStatus: {
    ready: '可用',
    training: '训练中',
    disabled: '已停用'
  },
  modelInstallStatus: {
    installed: '已安装',
    notInstalled: '未安装',
    failed: '安装失败'
  },
  historyTaskType: {
    modelTraining: '模型微调',
    textToSpeech: '文本转语音',
    voiceClone: '声音克隆',
    voiceDesign: '音色设计',
    streamingSpeech: '流式语音'
  },
  modelTraining: {
    sampleType: {
      single: '单样本',
      dataset: '样本集'
    }
  },
  textToSpeech: {
    format: {
      wav: 'WAV 无损',
      mp3: 'MP3 压缩',
      flac: 'FLAC 无损压缩'
    }
  },
  ui: {
    selectPlaceholder: '请选择',
    perPage: '{size} 条/页',
    totalItems: '共 {total} 条',
    range: '· 第 {start}-{end} 条',
    firstPage: '第一页',
    prevPage: '上一页',
    nextPage: '下一页',
    lastPage: '最后一页',
    tooltipFallback: '显示提示信息',
    defaultSpeaker: '默认说话人',
    continue: '继续',
    enabled: '已启用',
    disabled: '未启用',
    selectAudioFile: '选择音频文件',
    selectAudioButton: '选择音频',
    uploadText: '上传文本',
    selectTextFile: '选择文本文件',
    clear: '清空',
    audioFiles: '音频文件',
    textFiles: '文本文件',
    noAudioSelected: '尚未选择音频文件',
    noTextSelected: '尚未选择文本文件',
    emptyParamsTitle: '当前模型没有特有参数',
    emptyParamsDesc: '这个任务下没有额外参数需要配置，可以直接继续后续操作。'
  },
  audio: {
    downloadLabel: '下载音频',
    pendingMessage: '任务仍在执行中，音频结果会在状态变为“已完成”后显示。',
    failedMessage: '任务执行失败，无音频结果。可在历史任务详情中查看「任务日志」了解失败原因。',
    cancelledMessage: '任务已终止，无音频结果。',
    ended: '音频播放结束。',
    playFailed: '音频播放失败，请检查音频文件是否仍然可读。',
    playBlocked: '音频播放失败，当前环境可能阻止了播放',
    pause: '暂停播放',
    play: '播放音频',
    noResult: '当前还没有可播放的音频结果。',
    notCompleted: '当前任务尚未完成，完成后才可播放音频。',
    paused: '已暂停音频播放。',
    noDownload: '当前没有可下载的音频结果。',
    notCompletedDownload: '当前任务尚未完成，请等待状态更新后再下载。',
    downloadCancelled: '已取消下载。',
    saved: '音频已保存。',
    downloadFailed: '下载失败，请检查系统保存对话框权限和输出文件状态',
    decodeFailed: '音频播放失败，请检查音频数据是否可解码。',
    noPlayableIndex: '当前没有可播放的音频索引。',
    noDownloadIndex: '当前没有可下载的音频索引。',
    loadingAudio: '加载音频中',
    waitingData: '等待数据'
  },
  resultCard: {
    title: '生成结果',
    subtitle: '展示最近一次任务的返回结果和输出文件信息',
    loadingDetail: '载入中...',
    viewDetail: '查看详情',
    download: '下载'
  },
  recentList: {
    actionLabel: '查看'
  },
  deviceGuard: {
    title: '硬件环境确认',
    message: '当前选择硬件类型与模型环境当前支持的类型不一致，如果继续，会自动更新当前环境，不会影响任务正常执行，但是会大大延长本次任务执行的时间，是否继续？',
    checkFailed: '查询模型当前环境硬件类型失败',
    model: '模型：{model} {version}',
    currentEnvType: '当前环境类型：{device}',
    selectedType: '当前选择类型：{device}'
  },
  store: {
    models: {
      loadFailed: '加载模型列表失败',
      installed: '模型 {name} {version} 已安装。',
      installFailed: '安装模型失败',
      uninstalled: '模型 {name} {version} 已卸载。',
      uninstallFailed: '卸载模型失败',
      reinstallAfterUninstallFailed: '模型卸载失败，无法继续重装，请重试卸载。',
      reinstallAfterUninstall: '模型已卸载，但重装失败，请重试安装。',
      setCurrentDeviceFailed: '设置当前设备失败'
    },
    speakers: {
      loadFailed: '加载说话人列表失败',
      created: '已新增说话人“{name}”。',
      createFailed: '新增说话人失败',
      updated: '已更新说话人“{name}”的信息。',
      updateFailed: '保存说话人信息失败',
      imported: '已导入说话人“{name}”。',
      importFailed: '导入说话人失败',
      deleteFailed: '删除说话人失败。',
      deleted: '已删除说话人“{name}”。'
    },
    uiConfig: {
      loadFailed: '加载界面参数配置失败'
    },
    pagination: {
      loadFailed: '加载列表失败'
    }
  },
  languageName: {
    chinese: '中文',
    english: '英文',
    japanese: '日文',
    korean: '韩文'
  },
  notFound: {
    title: '页面不存在',
    description: '请从左侧导航返回有效页面。',
    backToTraining: '回到模型微调'
  }
};
