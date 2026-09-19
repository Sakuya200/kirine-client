export default {
  title: '说话人管理',
  description: '查看、筛选、编辑和删除已训练说话人。',
  stats: {
    total: '说话人总数',
    readyModels: '可用模型',
    training: '训练中',
    totalSamples: '样本总量'
  },
  panel: {
    title: '说话人列表',
    subtitle: '支持搜索、状态过滤、详情查看、编辑与删除操作',
    importModel: '导入模型',
    refresh: '刷新列表',
    refreshing: '刷新中...',
    searchPlaceholder: '搜索名称或备注',
    samples: '样本 {count} 条',
    createdAt: '创建于 {time} · 最近更新 {modifyTime}',
    viewDetail: '查看详情',
    edit: '编辑信息',
    delete: '删除',
    loading: '正在加载说话人列表...',
    empty: '当前筛选条件下没有匹配的说话人。'
  },
  detail: {
    title: '说话人详情',
    name: '名称：',
    model: '模型：',
    samples: '样本数：',
    status: '状态：',
    createTime: '创建时间：',
    modifyTime: '更新时间：',
    description: '备注：',
    close: '关闭'
  },
  editDialog: {
    title: '编辑说话人',
    name: '名称',
    namePlaceholder: '请输入说话人名称',
    description: '备注',
    descriptionPlaceholder: '请输入使用说明、适用场景或管理备注',
    cancel: '取消',
    save: '保存修改'
  },
  importDialog: {
    title: '导入外部模型',
    modelType: '模型类型',
    modelVersion: '模型版本',
    modelDir: '模型目录',
    dirPlaceholder: '请选择已下载模型所在目录',
    selectDir: '选择目录',
    dirPickerTitle: '选择已下载模型目录',
    speakerName: '说话人名称',
    speakerNamePlaceholder: '请输入说话人名称',
    speakerDescription: '说话人描述',
    speakerDescriptionPlaceholder: '请输入说话人描述或使用场景',
    cancel: '取消',
    confirm: '确认导入',
    importing: '导入中...'
  },
  deleteDialog: {
    title: '删除说话人',
    confirm: '将删除说话人“{name}”。该操作会执行逻辑删除，并从数据库查询结果中移除。',
    notFound: '未找到要删除的说话人。',
    cancel: '取消',
    confirmDelete: '确认删除',
    deleting: '删除中...'
  }
};
