export default {
  model: {
    deviceUnsupported: '模型 {model} 不支持设备 {device}，请切换为：{supported}',
    notInstalled: '请先在模型管理页安装后再执行任务（{model}）'
  },
  task: {
    handleReadFailed: '无法读取运行中任务句柄',
    terminateSignalFailed: '任务终止信号发送失败'
  },
  settings: {
    readStateFailed: '读取配置状态失败',
    writeStateFailed: '写入配置状态失败'
  },
  validation: {
    speakerNameRequired: '说话人名称不能为空',
    speakerDescriptionRequired: '说话人描述不能为空',
    baseModelTypeRequired: '基础模型类型不能为空',
    modelVersionRequired: '模型版本不能为空',
    refAudioRequired: '参考音频不能为空',
    textRequired: '目标台词不能为空',
    promptRequired: '音色描述不能为空'
  }
};
