<script setup lang="ts">
import { open as openFileDialog } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { ArrowDownTrayIcon, FolderOpenIcon, XMarkIcon } from '@heroicons/vue/24/outline';
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';

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
import { IMAGE_FILE_EXTENSIONS, type StreamingSpeakerConfig } from '@/types/streaming';
import type { SpeakerPagedResult, SpeakerProfile } from '@/types/domain';

interface Props {
  open: boolean;
  speaker?: StreamingSpeakerConfig | null;
  /** 会话运行中或历史回放中：模型已加载，运行中不支持新增已训练说话人。 */
  sessionLocked?: boolean;
}

const props = withDefaults(defineProps<Props>(), { speaker: null, sessionLocked: false });
const emit = defineEmits<{ close: []; submit: [payload: StreamingSpeakerInput] }>();

const modelStore = useModelStore();
const uiStore = useUiStore();
const { t } = useI18n();

const modelOptions = computed(() =>
  modelStore.getModelsByFeature(HistoryTaskType.StreamingSpeech).map(item => ({ label: item.modelName, value: item.baseModel }))
);

const categoryOptions = computed(() => {
  const options = [
    { label: t('streaming.form.categoryVoiceClone'), value: 'voice-clone' },
    { label: t('streaming.form.categoryTrained'), value: 'trained' }
  ];
  // 会话锁定时 trained 的 checkpoint 已随进程启动固化，不支持运行中新增已训练说话人
  if (props.sessionLocked && form.value.category !== 'trained') {
    return options.filter(option => option.value !== 'trained');
  }
  return options;
});

const sideOptions = computed(() => [
  { label: t('streaming.form.sideRight'), value: 'right' },
  { label: t('streaming.form.sideLeft'), value: 'left' }
]);

const createEmptyForm = (): StreamingSpeakerInput => ({
  name: '',
  baseModel: '',
  modelVersion: '',
  refAudioPath: '',
  refAudioName: '',
  refText: '',
  description: '',
  category: 'voice-clone',
  speakerDirName: '',
  side: 'right',
  avatarPath: '',
  avatarName: ''
});

const form = ref<StreamingSpeakerInput>(createEmptyForm());
const trainedSpeakers = ref<SpeakerProfile[]>([]);
// hydrate 期间抑制 category/baseModel watcher 的清空副作用，避免覆盖正在回填的值。
let isHydrating = false;

const isEditing = computed(() => Boolean(props.speaker));
const isTrained = computed(() => form.value.category === 'trained');
const needsRefText = computed(() => requiresRefText(form.value.baseModel));
// 编辑既有 trained 说话人时禁止更换 checkpoint（换 dir 相当于运行中换 trained，后端会拒绝）
const isTrainedDirLocked = computed(() => isEditing.value && isTrained.value);

const trainedSpeakerOptions = computed(() =>
  trainedSpeakers.value.filter(s => s.baseModel === form.value.baseModel).map(s => ({ label: s.speakerName, value: String(s.id) }))
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
    uiStore.notifyError(formatErrorMessage(t('streaming.form.loadTrainedFailed'), error));
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
      speakerDirName: speaker.speakerDirName ?? '',
      side: speaker.side ?? 'right',
      avatarPath: speaker.avatarPath ?? '',
      avatarName: speaker.avatarName ?? ''
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
      title: t('streaming.form.selectRefAudio'),
      multiple: false,
      directory: false,
      filters: [{ name: t('streaming.form.audioFiles'), extensions: [...MODEL_TRAINING_AUDIO_FILE_EXTENSIONS] }]
    });
    if (typeof selected === 'string' && selected.trim().length > 0) {
      const segments = selected.split(/[/\\]/);
      form.value.refAudioPath = selected;
      form.value.refAudioName = segments[segments.length - 1] ?? selected;
    }
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('streaming.form.pickerFailed'), error));
  }
};

const clearRefAudio = () => {
  form.value.refAudioPath = '';
  form.value.refAudioName = '';
};

const selectAvatar = async () => {
  try {
    const selected = await openFileDialog({
      title: t('streaming.form.selectAvatar'),
      multiple: false,
      directory: false,
      filters: [{ name: t('streaming.form.imageFiles'), extensions: [...IMAGE_FILE_EXTENSIONS] }]
    });
    if (typeof selected === 'string' && selected.trim().length > 0) {
      const segments = selected.split(/[/\\]/);
      form.value.avatarPath = selected;
      form.value.avatarName = segments[segments.length - 1] ?? selected;
    }
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('streaming.form.pickerFailed'), error));
  }
};

const clearAvatar = () => {
  form.value.avatarPath = '';
  form.value.avatarName = '';
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
    speakerDirName: isTrained.value ? form.value.speakerDirName : undefined,
    side: form.value.side ?? 'right',
    avatarPath: form.value.avatarPath || undefined,
    avatarName: form.value.avatarName || undefined
  });
};
</script>

<template>
  <BaseDialog
    :open="props.open"
    :title="isEditing ? t('streaming.form.editTitle') : t('streaming.form.addTitle')"
    content-class="max-h-[65vh] overflow-y-auto pr-2"
    @close="emit('close')"
  >
    <div class="space-y-4">
      <label class="block text-sm text-slate-700">
        <span class="mb-1 block text-xs text-stone-500">{{ t('streaming.form.name') }}</span>
        <input
          v-model="form.name"
          :disabled="isEditing"
          class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2 disabled:cursor-not-allowed disabled:bg-stone-50 disabled:text-stone-400"
          :placeholder="t('streaming.form.namePlaceholder')"
        />
        <span v-if="isEditing" class="mt-1 block text-xs text-stone-400">{{ t('streaming.form.nameLockedHint') }}</span>
      </label>

      <BaseListbox v-model="form.baseModel" :label="t('streaming.form.baseModel')" :options="modelOptions" :disabled="modelOptions.length === 0" />

      <BaseListbox v-model="form.category" :label="t('streaming.form.category')" :options="categoryOptions" />

      <BaseListbox v-model="form.side" :label="t('streaming.form.side')" :options="sideOptions" />

      <label class="block text-sm text-slate-700">
        <span class="mb-1 block text-xs text-stone-500">{{ t('streaming.form.avatar') }}</span>
        <div class="flex flex-wrap items-center gap-3 rounded-2xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700">
          <BaseButton tone="ghost" @click="selectAvatar">
            <FolderOpenIcon class="h-4 w-4" aria-hidden="true" />
            <span>{{ t('streaming.form.selectImage') }}</span>
          </BaseButton>
          <span class="min-w-0 flex-1 break-all">{{ form.avatarName || t('streaming.form.noAvatar') }}</span>
          <button v-if="form.avatarPath" type="button" class="text-xs text-stone-500 transition hover:text-brand-700" @click="clearAvatar">
            {{ t('streaming.form.clear') }}
          </button>
        </div>
      </label>

      <template v-if="isTrained">
        <BaseListbox
          v-model="form.speakerDirName"
          :label="t('streaming.form.trainedSpeaker')"
          :options="trainedSpeakerOptions"
          :disabled="isTrainedDirLocked || trainedSpeakerOptions.length === 0"
        />
        <p v-if="isTrainedDirLocked" class="text-xs text-stone-400">{{ t('streaming.form.trainedLockedHint') }}</p>
        <p v-else-if="trainedSpeakerOptions.length === 0" class="text-xs text-stone-400">{{ t('streaming.form.noTrainedSpeakers') }}</p>
      </template>

      <template v-else>
        <label class="block text-sm text-slate-700">
          <span class="mb-1 block text-xs text-stone-500">{{ t('streaming.form.refAudio') }}</span>
          <div class="flex flex-wrap items-center gap-3 rounded-2xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700">
            <BaseButton tone="ghost" @click="selectRefAudio">
              <FolderOpenIcon class="h-4 w-4" aria-hidden="true" />
              <span>{{ t('streaming.form.selectAudio') }}</span>
            </BaseButton>
            <span class="min-w-0 flex-1 break-all">{{ form.refAudioName || t('streaming.form.noRefAudio') }}</span>
            <button v-if="form.refAudioPath" type="button" class="text-xs text-stone-500 transition hover:text-brand-700" @click="clearRefAudio">
              {{ t('streaming.form.clear') }}
            </button>
          </div>
        </label>

        <label class="block text-sm text-slate-700">
          <span class="mb-1 block text-xs text-stone-500">
            {{ t('streaming.form.refText') }}<span v-if="needsRefText" class="text-rose-500">{{ t('streaming.form.required') }}</span><span v-else class="text-stone-400">{{ t('streaming.form.optional') }}</span>
          </span>
          <textarea
            v-model="form.refText"
            rows="3"
            class="w-full rounded-2xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700 outline-none transition focus:border-brand-400"
            :placeholder="needsRefText ? t('streaming.form.refTextRequiredPlaceholder') : t('streaming.form.refTextOptionalPlaceholder')"
          />
        </label>
      </template>

      <label class="block text-sm text-slate-700">
        <span class="mb-1 block text-xs text-stone-500">{{ t('streaming.form.description') }}</span>
        <textarea
          v-model="form.description"
          rows="2"
          class="w-full rounded-2xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700 outline-none transition focus:border-brand-400"
          :placeholder="t('streaming.form.descriptionPlaceholder')"
        />
      </label>

      <div class="rounded-xl border border-brand-100 bg-brand-50/40 px-3 py-2 text-xs text-stone-500">
        {{ t('streaming.form.summaryCategory') }}<span class="font-medium text-brand-700">{{ isTrained ? t('streaming.form.trained') : t('streaming.form.voiceClone') }}</span>
        <span v-if="isTrained">{{ t('streaming.form.trainedSummary') }}</span>
        <span v-else>{{ t('streaming.form.cloneSummary') }}</span>
      </div>
    </div>

    <template #footer>
      <BaseButton tone="ghost" @click="emit('close')">
        <XMarkIcon class="h-4 w-4" aria-hidden="true" />
        <span>{{ t('common.cancel') }}</span>
      </BaseButton>
      <BaseButton :disabled="!canSubmit" @click="submit">
        <ArrowDownTrayIcon v-if="!isEditing" class="h-4 w-4" aria-hidden="true" />
        <span>{{ isEditing ? t('streaming.form.save') : t('streaming.form.add') }}</span>
      </BaseButton>
    </template>
  </BaseDialog>
</template>
