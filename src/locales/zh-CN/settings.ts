export default {
  title: '设置',
  description: '管理服务连接、模型资源与数据日志路径。',
  panelTitle: '系统设置',
  uiLanguage: {
    label: '界面语言'
  },
  tabs: {
    connection: '连接配置',
    model: '模型资源',
    cache: '缓存配置'
  },
  connection: {
    serverUrl: 'Server URL',
    serverUrlPlaceholder: '请输入服务地址',
    apiToken: 'API Token',
    apiTokenPlaceholder: '请输入访问令牌',
    save: '保存连接配置'
  },
  model: {
    hint: '这里维护本地模型资源目录，以及统一的 Qwen 模型注意力实现。任务执行设备已经迁移到各任务页面与模型安装界面单独选择。',
    modelDir: '模型目录',
    modelDirPlaceholder: '请输入模型目录',
    attnLabel: '注意力实现',
    save: '保存资源配置'
  },
  cache: {
    header: '缓存配置',
    hint: '数据目录同时作为训练缓存与本地业务数据根目录，日志目录单独配置。',
    dataDir: '数据目录',
    dataDirPlaceholder: '请输入数据目录路径',
    logDir: '日志缓存路径',
    logDirPlaceholder: '请输入日志缓存路径',
    save: '保存缓存配置'
  },
  loading: {
    saving: '正在保存配置与迁移目录，请稍候',
    reading: '正在读取当前配置'
  },
  notice: {
    loaded: '已加载当前配置。',
    loadFailed: '读取配置失败，请检查 Rust 后端和配置文件',
    savedConnection: '连接配置已保存。',
    savedModel: '模型资源配置已保存。',
    savedCache: '缓存配置已保存。',
    saveFailed: '保存配置失败，请检查填写内容或后端日志',
    migrated: '已迁移{dirs}，请重启应用以切换到新目录。',
    cleanupHint: '旧目录内容仍保留，可在确认新目录正常后手动删除：{dirs}',
    cleanedHint: '模型旧目录内容已自动清理。'
  }
};
