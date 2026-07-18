<script setup lang="ts">
import { open as openFileDialog } from '@tauri-apps/plugin-dialog';
import { ArrowDownTrayIcon, FolderOpenIcon, XMarkIcon } from '@heroicons/vue/24/outline';
import { computed, ref, watch } from 'vue';

import BaseButton from '@/components/common/BaseButton.vue';
import BaseDialog from '@/components/common/BaseDialog.vue';
import BaseListbox from '@/components/common/BaseListbox.vue';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { MODEL_TRAINING_AUDIO_FILE_EXTENSIONS } from '@/enums/modelTraining';
import { HistoryTaskType } from '@/enums/task';
import { useModelStore } from '@/stores/models';
import { useUiStore } from '@/stores/ui';
import { requiresRefText, type StreamingSpeakerInput } from '@/stores/streamingSpeech';
import type { StreamingSpeakerConfig } from '@/types/streaming';

interface Props {
  open: boolean;
  speaker?: StreamingSpeakerConfig | null;
}

const props = withDefaults(defineProps<Props>(), { speaker: null });
const emit = defineEmits<{ close: []; submit: [payload: StreamingSpeakerInput] }>();

const modelStore = useModelStore();
const uiStore = useUiStore();

const modelOptions = computed(() =>
  modelStore.getModelsByFeature(HistoryTaskType.VoiceClone).map(item => ({ label: item.modelName, value: item.baseModel }))
);

const createEmptyForm = (): StreamingSpeakerInput => ({
  name: '',
  baseModel: '',
  modelVersion: '',
  refAudioPath: '',
  refAudioName: '',
  refText: '',
  description: ''
});

const form = ref<StreamingSpeakerInput>(createEmptyForm());

const isEditing = computed(() => Boolean(props.speaker));
const needsRefText = computed(() => requiresRefText(form.value.baseModel));
const canSubmit = computed(() => {
  const f = form.value;
  return (
    f.name.trim().length > 0 &&
    f.baseModel.length > 0 &&
    f.refAudioPath.length > 0 &&
    (!needsRefText.value || f.refText.trim().length > 0)
  );
});

const hydrateFromSpeaker = (speaker: StreamingSpeakerConfig | null) => {
  if (!speaker) {
    form.value = createEmptyForm();
    return;
  }
  form.value = {
    name: speaker.name,
    baseModel: speaker.baseModel,
    modelVersion: speaker.modelVersion,
    refAudioPath: speaker.refAudioPath,
    refAudioName: speaker.refAudioName,
    refText: speaker.refText,
    description: speaker.description ?? ''
  };
};

watch(
  () => props.open,
  next => {
    if (next) {
      hydrateFromSpeaker(props.speaker);
    }
  },
  { immediate: true }
);

watch(
  modelOptions,
  options => {
    if (options.length === 0) {
      return;
    }
    if (!options.some(option => option.value === form.value.baseModel)) {
      form.value.baseModel = String(options[0]?.value ?? '');
    }
  },
  { immediate: true }
);

const selectRefAudio = async () => {
  try {
    const selected = await openFileDialog({
      title: '选择参考音频',
      multiple: false,
      directory: false,
      filters: [{ name: '音频文件', extensions: [...MODEL_TRAINING_AUDIO_FILE_EXTENSIONS] }]
    });
    if (typeof selected === 'string' && selected.trim().length > 0) {
      const segments = selected.split(/[/\\]/);
      form.value.refAudioPath = selected;
      form.value.refAudioName = segments[segments.length - 1] ?? selected;
    }
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('打开文件选择器失败', error));
  }
};

const clearRefAudio = () => {
  form.value.refAudioPath = '';
  form.value.refAudioName = '';
};

const submit = () => {
  if (!canSubmit.value) {
    return;
  }
  emit('submit', {
    name: form.value.name.trim(),
    baseModel: form.value.baseModel,
    modelVersion: form.value.modelVersion,
    refAudioPath: form.value.refAudioPath,
    refAudioName: form.value.refAudioName,
    refText: form.value.refText.trim(),
    description: form.value.description?.trim() || undefined
  });
};
</script>

<template>
  <BaseDialog :open="props.open" :title="isEditing ? '编辑说话人' : '新增说话人'" @close="emit('close')">
    <div class="space-y-4">
      <label class="block text-sm text-slate-700">
        <span class="mb-1 block text-xs text-stone-500">说话人名称</span>
        <input
          v-model="form.name"
          class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
          placeholder="为该说话人起个名字"
        />
      </label>

      <BaseListbox v-model="form.baseModel" label="对应模型" :options="modelOptions" :disabled="modelOptions.length === 0" />

      <label class="block text-sm text-slate-700">
        <span class="mb-1 block text-xs text-stone-500">参考音频</span>
        <div class="flex flex-wrap items-center gap-3 rounded-2xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700">
          <BaseButton tone="ghost" @click="selectRefAudio">
            <FolderOpenIcon class="h-4 w-4" aria-hidden="true" />
            <span>选择音频</span>
          </BaseButton>
          <span class="min-w-0 flex-1 break-all">{{ form.refAudioName || '尚未选择参考音频' }}</span>
          <button v-if="form.refAudioPath" type="button" class="text-xs text-stone-500 transition hover:text-brand-700" @click="clearRefAudio">
            清空
          </button>
        </div>
      </label>

      <label class="block text-sm text-slate-700">
        <span class="mb-1 block text-xs text-stone-500">
          参考台词<span v-if="needsRefText" class="text-rose-500"> *必填</span><span v-else class="text-stone-400">（可选）</span>
        </span>
        <textarea
          v-model="form.refText"
          rows="3"
          class="w-full rounded-2xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700 outline-none transition focus:border-brand-400"
          :placeholder="needsRefText ? '当前模型需要参考文本，请填写参考音频中实际说出的内容' : '可选，填写参考音频中实际说出的文本'"
        />
      </label>

      <label class="block text-sm text-slate-700">
        <span class="mb-1 block text-xs text-stone-500">备注（可选）</span>
        <textarea
          v-model="form.description"
          rows="2"
          class="w-full rounded-2xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700 outline-none transition focus:border-brand-400"
          placeholder="可填写使用场景或管理备注"
        />
      </label>

      <div class="rounded-xl border border-brand-100 bg-brand-50/40 px-3 py-2 text-xs text-stone-500">
        类别：<span class="font-medium text-brand-700">语音克隆</span>（本期仅支持此类别，未来将支持更多来源）
      </div>
    </div>

    <template #footer>
      <BaseButton tone="ghost" @click="emit('close')">
        <XMarkIcon class="h-4 w-4" aria-hidden="true" />
        <span>取消</span>
      </BaseButton>
      <BaseButton :disabled="!canSubmit" @click="submit">
        <ArrowDownTrayIcon v-if="!isEditing" class="h-4 w-4" aria-hidden="true" />
        <span>{{ isEditing ? '保存修改' : '添加说话人' }}</span>
      </BaseButton>
    </template>
  </BaseDialog>
</template>
