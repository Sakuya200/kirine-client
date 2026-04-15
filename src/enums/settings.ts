export enum BaseModel {
  Qwen3Tts = 'qwen3_tts'
}

export enum HardwareType {
  Cpu = 'cpu',
  Cuda = 'cuda'
}

export enum AttentionImplementation {
  Sdpa = 'sdpa',
  FlashAttention2 = 'flash_attention_2',
  Eager = 'eager'
}

export enum QloraMode {
  Enabled = 'enabled',
  Disabled = 'disabled'
}

export enum QloraQuantType {
  Nf4 = 'nf4',
  Fp4 = 'fp4'
}

export const BASE_MODEL_TEXT: Record<BaseModel, string> = {
  [BaseModel.Qwen3Tts]: 'Qwen3-TTS'
};

export const HARDWARE_TYPE_TEXT: Record<HardwareType, string> = {
  [HardwareType.Cpu]: 'CPU',
  [HardwareType.Cuda]: 'CUDA'
};

export const ATTENTION_IMPLEMENTATION_TEXT: Record<AttentionImplementation, string> = {
  [AttentionImplementation.Sdpa]: 'SDPA',
  [AttentionImplementation.FlashAttention2]: 'Flash Attention 2',
  [AttentionImplementation.Eager]: 'Eager'
};

export const QLORA_MODE_TEXT: Record<QloraMode, string> = {
  [QloraMode.Enabled]: '启用',
  [QloraMode.Disabled]: '禁用'
};

export const QLORA_QUANT_TYPE_TEXT: Record<QloraQuantType, string> = {
  [QloraQuantType.Nf4]: 'NF4',
  [QloraQuantType.Fp4]: 'FP4'
};
