export default {
  title: '模型管理',
  description: '查看系统支持的基础模型、功能支持情况和当前安装状态，并执行安装或卸载。',
  panel: {
    title: '模型列表',
    subtitle: '系统中可用的基础模型及其支持的功能和安装状态',
    refresh: '刷新列表',
    refreshing: '刷新中...',
    loading: '正在加载模型列表...',
    empty: '当前没有可展示的模型信息。'
  },
  busy: {
    mutating: '正在处理模型安装或卸载，请稍候',
    loading: '正在加载模型列表'
  },
  table: {
    model: '模型',
    version: '版本',
    features: '支持功能',
    dependencies: '依赖',
    currentDevice: '当前设备',
    status: '状态',
    actions: '操作',
    devicePlaceholder: '请选择设备',
    deviceRequired: '请先选择当前设备',
    reinstalling: '重装中...',
    installing: '安装中...',
    reinstall: '重装',
    install: '安装',
    uninstall: '卸载'
  },
  deleteDialog: {
    title: '卸载模型',
    confirm: '将卸载模型“{name} {version}”的专属权重文件，并把状态改为未安装。共享依赖会保留。',
    confirmDelete: '确认卸载',
    notFound: '未找到要卸载的模型。',
    uninstalling: '卸载中...'
  }
};
