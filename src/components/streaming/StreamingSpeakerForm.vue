<script setup lang="ts">
import { open as openFileDialog } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { ArrowDownTrayIcon, FolderOpenIcon, XMarkIcon } from '@heroicons/vue/24/outline';
import { computed, ref, watch } from 'vue';

import BaseButton from '@/components/common/BaseButton.vue';
import BaseDialog from '@/components/common/BaseDialog.vue';
import BaseListbox from '@/components/common/BaseListbox.vue';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { MODEL_TRAINING_AUDIO_FILE_EXTENSIONS } from '@/enums/modelTraining';
import { SpeakerStatus } from '@/enums/status';
import { HistoryTaskType } from '@/enums/task';
import { useModelStore } from '@/stores/models';
import { useUiStore } from '@/stores/ui';
import { requiresRefText, type StreamingSpeakerInput } from '@/stores/streamingSpeech';
import type { StreamingSpeakerConfig } from '@/types/streaming';
import type { SpeakerPagedResult, SpeakerProfile } from '@/types/domain';

interface Props {
  open: boolean;
  speaker?: StreamingSpeakerConfig | null;
}

const props = withDefaults(defineProps<Props>(), { speaker: null });
const emit = defineEmits<{ close: []; submit: [payload: StreamingSpeakerInput] }>();

const modelStore = useModelStore();
const uiStore = useUiStore();

const modelOptions = computed(() =>
  modelStore.getModelsByFeature(HistoryTaskType.StreamingSpeech).map(item => ({ label: item.modelName, value: item.baseModel }))
);

const categoryOptions = [
  { label: '语音克隆', value: 'voice-clone' },
  { label: '已训练', value: 'trained' }
];

const createEmptyForm = (): StreamingSpeakerInput => ({
  name: '',
  baseModel: '',
  modelVersion: '',
  refAudioPath: '',
  refAudioName: '',
  refText: '',
  description: '',
  category: 'voice-clone',
  speakerDirName: ''
});

const form = ref<StreamingSpeakerInput>(createEmptyForm());
const trainedSpeakers = ref<SpeakerProfile[]>([]);
// hydrate 期间抑制 category/baseModel watcher 的清空副作用，避免覆盖正在回填的值。
let isHydrating = false;

const isEditing = computed(() => Boolean(props.speaker));
const isTrained = computed(() => form.value.category === 'trained');
const needsRefText = computed(() => requiresRefText(form.value.baseModel));

const trainedSpeakerOptions = computed(() =>
  trainedSpeakers.value
    .filter(s => s.baseModel === form.value.baseModel)
    .map(s => ({ label: s.speakerName, value: String(s.id) }))
);

const canSubmit = computed(() => {
  const f = form.value;
  if (f.name.trim().length === 0 || f.baseModel.length === 0) {
    return false;
  }
  if (isTrained.value) {
    return (f.speakerDirName ?? '').length > 0;
  }
  return f.refAudioPath.length > 0 && (!needsRefText.value || f.refText.trim().length > 0);
});

const loadTrainedSpeakers = async () => {
  try {
    const result = await invoke<SpeakerPagedResult>('list_speaker_infos', {
      request: { page: 1, pageSize: 1000, filter: { status: SpeakerStatus.Ready } }
    });
    trainedSpeakers.value = result.items;
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('加载已训练说话人失败', error));
  }
};

const hydrateFromSpeaker = (speaker: StreamingSpeakerConfig | null) => {
  isHydrating = true;
  try {
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
      description: speaker.description ?? '',
      category: speaker.category,
      speakerDirName: speaker.speakerDirName ?? ''
    };
    if (speaker.category === 'trained') {
      loadTrainedSpeakers();
    }
  } finally {
    isHydrating = false;
  }
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

// 切换类别：trained 清空 ref 音频/台词并加载已训练说话人；voice-clone 清空 speakerDirName。
// flush:'sync' + isHydrating guard：确保 hydrate 回填的值不被清空。
watch(
  () => form.value.category,
  cat => {
    if (isHydrating) {
      return;
    }
    if (cat === 'trained') {
      form.value.refAudioPath = '';
      form.value.refAudioName = '';
      form.value.refText = '';
      form.value.speakerDirName = '';
      loadTrainedSpeakers();
    } else {
      form.value.speakerDirName = '';
    }
  },
  { flush: 'sync' }
);

// trained 模式下切换模型时重新加载已训练说话人列表，并清空失效的 speakerDirName。
watch(
  () => form.value.baseModel,
  () => {
    if (isHydrating) {
      return;
    }
    if (isTrained.value) {
      loadTrainedSpeakers();
    }
    form.value.speakerDirName = '';
  },
  { flush: 'sync' }
);

// 选中已训练说话人时，名称为空则自动填充。
watch(
  () => form.value.speakerDirName,
  val => {
    if (!isTrained.value || !val) {
      return;
    }
    const spk = trainedSpeakers.value.find(s => String(s.id) === val);
    if (spk && !form.value.name.trim()) {
      form.value.name = spk.speakerName;
    }
  }
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
    description: form.value.description?.trim() || undefined,
    category: form.value.category,
    speakerDirName: isTrained.value ? form.value.speakerDirName : undefined
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

      <BaseListbox v-model="form.category" label="说话人类别" :options="categoryOptions" />

      <template v-if="isTrained">
        <BaseListbox
          v-model="form.speakerDirName"
          label="已训练说话人"
          :options="trainedSpeakerOptions"
          :disabled="trainedSpeakerOptions.length === 0"
        />
        <p v-if="trainedSpeakerOptions.length === 0" class="text-xs text-stone-400">
          当前模型暂无可用已训练说话人，请先在模型微调页完成一次微调。
        </p>
      </template>

      <template v-else>
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
      </template>

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
        类别：<span class="font-medium text-brand-700">{{ isTrained ? '已训练' : '语音克隆' }}</span>
        <span v-if="isTrained">（使用微调产出的 checkpoint 合成，无需参考音频）</span>
        <span v-else>（使用基座模型 + 参考音频克隆音色）</span>
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
