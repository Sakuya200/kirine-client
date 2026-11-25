<script setup lang="ts">
import { open as openFileDialog } from '@tauri-apps/plugin-dialog';
import { ArrowDownTrayIcon, FolderOpenIcon, XMarkIcon } from '@heroicons/vue/24/outline';
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';

import BaseButton from '@/components/common/BaseButton.vue';
import BaseDialog from '@/components/common/BaseDialog.vue';
import BaseListbox from '@/components/common/BaseListbox.vue';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { MODEL_TRAINING_AUDIO_FILE_EXTENSIONS } from '@/enums/modelTraining';
import { useSpeakerStore } from '@/stores/speakers';
import { useUiStore } from '@/stores/ui';
import { useStreamingSpeechStore } from '@/stores/streamingSpeech';
import { requiresRefText, type StreamingSpeakerInput } from '@/stores/streamingSpeech';
import { IMAGE_FILE_EXTENSIONS, type StreamingSpeakerConfig } from '@/types/streaming';
import type { SpeakerProfile } from '@/types/domain';

interface Props {
  open: boolean;
  speaker?: StreamingSpeakerConfig | null;
  /** 会话运行中（进程已拉起）：模型 checkpoint 已加载，运行中不支持新增已训练说话人；
   * 历史回放/已结束会话不锁定（后端仅落盘，下次会话启动重新加载）。 */
  sessionLocked?: boolean;
}

const props = withDefaults(defineProps<Props>(), { speaker: null, sessionLocked: false });
const emit = defineEmits<{ close: []; submit: [payload: StreamingSpeakerInput] }>();

const uiStore = useUiStore();
const speakerStore = useSpeakerStore();
const streamingStore = useStreamingSpeechStore();
const { t } = useI18n();

// 说话人不再自带模型配置：统一跟随外层会话模型（抽屉中的模型选择）。
const sessionBaseModel = computed(() => streamingStore.sessionConfig.baseModel);

const categoryOptions = computed(() => {
  const options = [
    { label: t('streaming.form.categoryVoiceClone'), value: 'voice-clone' },
    { label: t('streaming.form.categoryPreset'), value: 'preset' },
    { label: t('streaming.form.categoryTrained'), value: 'trained' }
  ];
  // 会话锁定时 trained 的 checkpoint 已随进程启动固化，不支持运行中新增已训练说话人；
  // preset 与 voice-clone 均可运行中增删改。
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
  modelVersion: '',
  refAudioPath: '',
  refAudioName: '',
  refText: '',
  description: '',
  category: 'voice-clone',
  speakerDirName: '',
  side: 'right',
  avatarPath: '',
  avatarName: '',
  speakerId: null,
  avatarContentType: null
});

const form = ref<StreamingSpeakerInput>(createEmptyForm());
const readySpeakers = ref<SpeakerProfile[]>([]);
// preset 分支当前选中的内置音色（= form.name，单独 ref 以驱动 Listbox）。
const selectedPreset = ref('');
// hydrate 期间抑制 category watcher 的清空副作用，避免覆盖正在回填的值。
let isHydrating = false;

const isEditing = computed(() => Boolean(props.speaker));
const isTrained = computed(() => form.value.category === 'trained');
const isPreset = computed(() => form.value.category === 'preset');
const needsRefText = computed(() => requiresRefText(sessionBaseModel.value));
// 编辑既有 trained 说话人时禁止更换 checkpoint（换 dir 相当于运行中换 trained，后端会拒绝）
const isTrainedDirLocked = computed(() => isEditing.value && isTrained.value);

// trained 下拉只显示本地训练产出的 Ready 说话人：preset 音色（source='preset'）
// 入库后同样 status=Ready，必须按 source 过滤避免混入 trained 分支。
// 模型统一跟随会话配置，按会话模型过滤（checkpoint 按会话模型加载）。
const trainedSpeakerOptions = computed(() =>
  readySpeakers.value
    .filter(s => s.source === 'local' && s.baseModel === sessionBaseModel.value)
    .map(s => ({ label: s.speakerName, value: String(s.id) }))
);

// preset 下拉：模型预置音色（启动时由后端从 model-config 同步入库）。
const presetSpeakerOptions = computed(() =>
  readySpeakers.value
    .filter(s => s.source === 'preset' && s.baseModel === sessionBaseModel.value)
    .map(s => ({ label: s.speakerName, value: s.speakerName }))
);

const canSubmit = computed(() => {
  const f = form.value;
  if (f.name.trim().length === 0) {
    return false;
  }
  if (isTrained.value) {
    return (f.speakerDirName ?? '').length > 0;
  }
  if (isPreset.value) {
    // 名称由内置音色选中项自动填充，必须精确匹配预置名。
    return presetSpeakerOptions.value.some(option => option.value === f.name.trim());
  }
  return f.refAudioPath.length > 0 && (!needsRefText.value || f.refText.trim().length > 0);
});

const loadReadySpeakers = async () => {
  try {
    readySpeakers.value = await speakerStore.listReadySpeakers({ baseModel: sessionBaseModel.value || null });
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('streaming.form.loadSpeakersFailed'), error));
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
      modelVersion: speaker.modelVersion,
      refAudioPath: speaker.refAudioPath,
      refAudioName: speaker.refAudioName,
      refText: speaker.refText,
      description: speaker.description ?? '',
      category: speaker.category,
      speakerDirName: speaker.speakerDirName ?? '',
      side: speaker.side ?? 'right',
      avatarPath: speaker.avatarPath ?? '',
      avatarName: speaker.avatarName ?? '',
      speakerId: speaker.speakerId ?? null,
      avatarContentType: speaker.avatarContentType ?? null
    };
    if (speaker.category === 'trained') {
      loadReadySpeakers();
    }
    if (speaker.category === 'preset') {
      selectedPreset.value = speaker.name;
      loadReadySpeakers();
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
  () => props.open,
  next => {
    if (next) {
      hydrateFromSpeaker(props.speaker);
    }
  },
  { immediate: true }
);

// 切换类别：trained/preset 清空 ref 音频/台词并加载 Ready 说话人列表；voice-clone 清空
// speakerDirName 与说话人记录关联（记录头像随 speakerId 一并失效）；preset 另清空选中音色与名称。
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
      loadReadySpeakers();
    } else if (cat === 'preset') {
      form.value.refAudioPath = '';
      form.value.refAudioName = '';
      form.value.refText = '';
      form.value.speakerDirName = '';
      selectedPreset.value = '';
      form.value.name = '';
      loadReadySpeakers();
    } else {
      form.value.speakerDirName = '';
      form.value.speakerId = null;
      form.value.avatarContentType = null;
      selectedPreset.value = '';
    }
  },
  { flush: 'sync' }
);

// 会话模型切换（抽屉中的模型选择）时：旧模型的说话人列表失效，重新加载并清空
// 失效的关联字段（trained checkpoint 按会话模型加载；preset 音色按模型预置）。
watch(
  sessionBaseModel,
  () => {
    if (isHydrating) {
      return;
    }
    if (isTrained.value || isPreset.value) {
      loadReadySpeakers();
    }
    if (isPreset.value) {
      selectedPreset.value = '';
      form.value.name = '';
    }
    form.value.speakerDirName = '';
    form.value.speakerId = null;
    form.value.avatarContentType = null;
  },
  { flush: 'sync' }
);

// 选中已训练说话人时，名称为空则自动填充，并带出记录关联（id + 记录头像 MIME）。
// 记录头像即默认头像（avatarPath 为空表示未覆盖）；用户手动选图则为会话覆盖。
watch(
  () => form.value.speakerDirName,
  val => {
    if (!isTrained.value || !val) {
      return;
    }
    const spk = readySpeakers.value.find(s => String(s.id) === val);
    if (!spk) {
      return;
    }
    if (!form.value.name.trim()) {
      form.value.name = spk.speakerName;
    }
    form.value.speakerId = spk.id;
    form.value.avatarContentType = spk.avatarContentType;
  }
);

// 选中内置音色时：名称必须精确匹配预置名（即 API 的 speaker 参数），备注为空则
// 自动带出预置描述，并关联 speakers 记录（id + 记录头像 MIME，头像回退链路自动生效）。
watch(selectedPreset, val => {
  if (!isPreset.value || !val) {
    return;
  }
  const spk = readySpeakers.value.find(s => s.source === 'preset' && s.speakerName === val);
  if (!spk) {
    return;
  }
  form.value.name = spk.speakerName;
  if (!form.value.description?.trim()) {
    form.value.description = spk.description ?? '';
  }
  form.value.speakerId = spk.id;
  form.value.avatarContentType = spk.avatarContentType;
});

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

// 清除会话头像覆盖：有记录头像时回退记录头像（speakerId/avatarContentType 保持），
// 无记录关联时即为清空。
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
    modelVersion: form.value.modelVersion,
    refAudioPath: form.value.refAudioPath,
    refAudioName: form.value.refAudioName,
    refText: form.value.refText.trim(),
    description: form.value.description?.trim() || undefined,
    category: form.value.category,
    speakerDirName: isTrained.value ? form.value.speakerDirName : undefined,
    side: form.value.side ?? 'right',
    avatarPath: form.value.avatarPath || undefined,
    avatarName: form.value.avatarName || undefined,
    speakerId: form.value.speakerId ?? undefined
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
          :disabled="isEditing || isPreset"
          class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2 disabled:cursor-not-allowed disabled:bg-stone-50 disabled:text-stone-400"
          :placeholder="isPreset ? t('streaming.form.presetNamePlaceholder') : t('streaming.form.namePlaceholder')"
        />
        <span v-if="isEditing" class="mt-1 block text-xs text-stone-400">{{ t('streaming.form.nameLockedHint') }}</span>
      </label>

      <BaseListbox v-model="form.category" :label="t('streaming.form.category')" :options="categoryOptions" />

      <BaseListbox v-model="form.side" :label="t('streaming.form.side')" :options="sideOptions" />

      <label class="block text-sm text-slate-700">
        <span class="mb-1 block text-xs text-stone-500">{{ t('streaming.form.avatar') }}</span>
        <div class="flex flex-wrap items-center gap-3 rounded-2xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700">
          <BaseButton tone="ghost" @click="selectAvatar">
            <FolderOpenIcon class="h-4 w-4" aria-hidden="true" />
            <span>{{ t('streaming.form.selectImage') }}</span>
          </BaseButton>
          <span class="min-w-0 flex-1 break-all">
            {{ form.avatarName || (form.avatarContentType ? t('streaming.form.recordAvatar') : t('streaming.form.noAvatar')) }}
          </span>
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

      <template v-else-if="isPreset">
        <BaseListbox
          v-model="selectedPreset"
          :label="t('streaming.form.presetSpeaker')"
          :options="presetSpeakerOptions"
          :disabled="presetSpeakerOptions.length === 0"
        />
        <p v-if="presetSpeakerOptions.length === 0" class="text-xs text-stone-400">{{ t('streaming.form.noPresetSpeakers') }}</p>
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
        {{ t('streaming.form.summaryCategory') }}<span class="font-medium text-brand-700">{{ isTrained ? t('streaming.form.trained') : isPreset ? t('streaming.form.preset') : t('streaming.form.voiceClone') }}</span>
        <span v-if="isTrained">{{ t('streaming.form.trainedSummary') }}</span>
        <span v-else-if="isPreset">{{ t('streaming.form.presetSummary') }}</span>
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
