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

export enum LoraMode {
  Enabled = 'enabled',
  Disabled = 'disabled'
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

export const LORA_MODE_TEXT: Record<LoraMode, string> = {
  [LoraMode.Enabled]: '启用',
  [LoraMode.Disabled]: '禁用'
};
