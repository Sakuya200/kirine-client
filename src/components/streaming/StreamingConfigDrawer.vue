<script setup lang="ts">
import { PencilSquareIcon, PlusIcon, TrashIcon, XMarkIcon } from '@heroicons/vue/24/outline';
import { computed, onBeforeUnmount, ref, watch } from 'vue';

import BaseButton from '@/components/common/BaseButton.vue';
import BaseListbox from '@/components/common/BaseListbox.vue';
import PanelCard from '@/components/common/PanelCard.vue';
import GenericTaskParamsForm from '@/components/form/GenericTaskParamsForm.vue';
import StreamingSpeakerForm from '@/components/streaming/StreamingSpeakerForm.vue';
import { APP_LANGUAGE_LABELS, AppLanguage } from '@/enums/language';
import { HARDWARE_TYPE_TEXT, HardwareType } from '@/enums/settings';
import { HistoryTaskType } from '@/enums/task';
import { useModelStore } from '@/stores/models';
import { useStreamingSpeechStore, type StreamingSpeakerInput } from '@/stores/streamingSpeech';
import { useUiConfigStore } from '@/stores/uiConfig';
import { useUiStore } from '@/stores/ui';
import { mergeModelParamsWithUiConfigDefaults } from '@/utils/uiConfigModelParams';
import type { StreamingSpeakerConfig } from '@/types/streaming';
import type { TaskParamConfig } from '@/types/uiConfig';

interface Props {
  open: boolean;
  width?: number;
}

const props = withDefaults(defineProps<Props>(), { width: 380 });
const emit = defineEmits<{ close: []; 'update:width': [value: number] }>();

const store = useStreamingSpeechStore();
const modelStore = useModelStore();
const uiConfigStore = useUiConfigStore();
const uiStore = useUiStore();

const drawerWidth = ref(props.width);
const isFormOpen = ref(false);
const editingSpeaker = ref<StreamingSpeakerConfig | null>(null);

const modelOptions = computed(() =>
  modelStore.getModelsByFeature(HistoryTaskType.VoiceClone).map(item => ({ label: item.modelName, value: item.baseModel }))
);
const modelVersionOptions = computed(() => modelStore.getModelVersionOptions(store.sessionConfig.baseModel));
const deviceOptions = computed(() =>
  modelStore.getSupportedDevices(store.sessionConfig.baseModel, store.sessionConfig.modelVersion).map(device => ({
    value: device,
    label: HARDWARE_TYPE_TEXT[device as HardwareType] ?? device.toUpperCase()
  }))
);
const languageOptions = computed(() =>
  modelStore.getSupportedLanguages(store.sessionConfig.baseModel, store.sessionConfig.modelVersion).map(language => ({
    value: language,
    label: APP_LANGUAGE_LABELS[language] ?? language
  }))
);
const activeTaskConfig = computed<TaskParamConfig | null>(() => uiConfigStore.getTaskConfig(store.sessionConfig.baseModel, HistoryTaskType.VoiceClone));

const normalizeModelParams = (baseModel: string, modelParams: Record<string, unknown>) => {
  const taskConfig = uiConfigStore.getTaskConfig(baseModel, HistoryTaskType.VoiceClone);
  return mergeModelParamsWithUiConfigDefaults(taskConfig, modelParams);
};

// 选项同步默认值（镜像 VoiceCloneView 的 watch 模式）
watch(
  modelOptions,
  options => {
    if (options.length === 0) {
      return;
    }
    if (!options.some(option => option.value === store.sessionConfig.baseModel)) {
      store.setSessionConfig({ baseModel: String(options[0]?.value ?? '') });
    }
  },
  { immediate: true }
);

watch(
  modelVersionOptions,
  options => {
    if (options.length === 0) {
      store.setSessionConfig({ modelVersion: '' });
      return;
    }
    if (!options.some(option => option.value === store.sessionConfig.modelVersion)) {
      store.setSessionConfig({ modelVersion: String(options[0]?.value ?? '') });
    }
  },
  { immediate: true }
);

watch(
  deviceOptions,
  options => {
    if (options.length === 0) {
      store.setSessionConfig({ device: HardwareType.Cpu });
      return;
    }
    const matched = options.find(option => option.value === store.sessionConfig.device) ?? options[0];
    store.setSessionConfig({ device: (matched?.value ?? HardwareType.Cpu) as string });
  },
  { immediate: true }
);

watch(
  languageOptions,
  options => {
    if (options.length === 0) {
      store.setSessionConfig({ language: AppLanguage.Chinese });
      return;
    }
    const matched = options.find(option => option.value === store.sessionConfig.language) ?? options[0];
    store.setSessionConfig({ language: (matched?.value ?? AppLanguage.Chinese) as AppLanguage });
  },
  { immediate: true }
);

watch(
  () => store.sessionConfig.baseModel,
  nextBaseModel => {
    store.setSessionConfig({ modelParams: normalizeModelParams(nextBaseModel, store.sessionConfig.modelParams) });
  },
  { immediate: true }
);

// 选项变更 handler（避免在模板里写类型断言）
const onBaseModelChange = (value: unknown) => store.setSessionConfig({ baseModel: String(value) });
const onModelVersionChange = (value: unknown) => store.setSessionConfig({ modelVersion: String(value) });
const onDeviceChange = (value: unknown) => store.setSessionConfig({ device: String(value) });
const onLanguageChange = (value: unknown) => store.setSessionConfig({ language: value as AppLanguage });
const onModelParamsChange = (value: Record<string, unknown>) => store.setSessionConfig({ modelParams: value });

// 说话人管理
const openAddSpeaker = () => {
  editingSpeaker.value = null;
  isFormOpen.value = true;
};
const openEditSpeaker = (speaker: StreamingSpeakerConfig) => {
  editingSpeaker.value = speaker;
  isFormOpen.value = true;
};
const closeSpeakerForm = () => {
  isFormOpen.value = false;
  editingSpeaker.value = null;
};
const submitSpeaker = (payload: StreamingSpeakerInput) => {
  if (editingSpeaker.value) {
    store.updateSpeaker(editingSpeaker.value.id, payload);
    uiStore.notifySuccess(`已更新说话人「${payload.name}」。`, 3200);
  } else {
    store.addSpeaker(payload);
    uiStore.notifySuccess(`已添加说话人「${payload.name}」。`, 3200);
  }
  closeSpeakerForm();
};
const removeSpeaker = (speaker: StreamingSpeakerConfig) => {
  store.removeSpeaker(speaker.id);
  uiStore.notifyInfo(`已移除说话人「${speaker.name}」。`, 2200);
};

const categoryLabel = (speaker: StreamingSpeakerConfig) => (speaker.category === 'voice-clone' ? '语音克隆' : speaker.category);

// 左边缘拖拽调宽（320–560）
const startDrag = (event: PointerEvent) => {
  event.preventDefault();
  const startX = event.clientX;
  const startW = drawerWidth.value;
  document.body.style.cursor = 'col-resize';
  document.body.style.userSelect = 'none';
  const onMove = (ev: PointerEvent) => {
    const next = Math.min(560, Math.max(320, startW - (ev.clientX - startX)));
    drawerWidth.value = next;
    emit('update:width', next);
  };
  const onUp = () => {
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
    document.removeEventListener('pointermove', onMove);
    document.removeEventListener('pointerup', onUp);
  };
  document.addEventListener('pointermove', onMove);
  document.addEventListener('pointerup', onUp);
};

onBeforeUnmount(() => {
  document.body.style.cursor = '';
  document.body.style.userSelect = '';
});
</script>

<template>
  <Teleport to="body">
    <Transition name="streaming-drawer">
      <div v-if="open" class="fixed inset-0 z-[110]" role="dialog" aria-modal="true">
        <div class="absolute inset-0 bg-[#7a4a24]/18 backdrop-blur-[2px]" @click="emit('close')" />
        <aside
          class="absolute inset-y-0 right-0 flex flex-col border-l border-brand-100 bg-[#fffdfa] shadow-panel"
          :style="{ width: `${drawerWidth}px` }"
        >
          <div
            class="absolute left-0 top-0 z-10 h-full w-1.5 cursor-col-resize bg-transparent transition hover:bg-brand-200/60"
            @pointerdown="startDrag"
          />

          <header class="flex items-center justify-between gap-3 border-b border-brand-100 px-5 py-4">
            <div>
              <p class="text-[11px] uppercase tracking-[0.26em] text-brand-500">Streaming</p>
              <h2 class="text-base font-semibold text-slate-900">流式语音配置</h2>
            </div>
            <button
              class="inline-flex h-9 w-9 items-center justify-center rounded-lg text-stone-500 transition hover:bg-brand-50 hover:text-brand-700"
              @click="emit('close')"
            >
              <XMarkIcon class="h-5 w-5" aria-hidden="true" />
            </button>
          </header>

          <div class="flex-1 space-y-4 overflow-y-auto p-5">
            <PanelCard title="基础配置" subtitle="选择模型、设备与输出语言">
              <div class="grid gap-4 md:grid-cols-2">
                <BaseListbox :model-value="store.sessionConfig.baseModel" label="基础模型" :options="modelOptions" @update:model-value="onBaseModelChange" />
                <BaseListbox
                  :model-value="store.sessionConfig.modelVersion"
                  label="模型版本"
                  :options="modelVersionOptions"
                  :disabled="modelVersionOptions.length === 0"
                  @update:model-value="onModelVersionChange"
                />
                <BaseListbox :model-value="store.sessionConfig.device" label="设备类型" :options="deviceOptions" :disabled="deviceOptions.length === 0" @update:model-value="onDeviceChange" />
                <BaseListbox :model-value="store.sessionConfig.language" label="输出语言" :options="languageOptions" @update:model-value="onLanguageChange" />
              </div>
            </PanelCard>

            <PanelCard title="说话人管理" subtitle="配置可在聊天中选择的语音克隆说话人">
              <template #actions>
                <BaseButton tone="ghost" size="sm" @click="openAddSpeaker">
                  <PlusIcon class="h-4 w-4" aria-hidden="true" />
                  <span>新增</span>
                </BaseButton>
              </template>

              <div v-if="store.speakers.length > 0" class="space-y-2">
                <article v-for="speaker in store.speakers" :key="speaker.id" class="rounded-2xl border border-brand-200 bg-white/90 p-3">
                  <div class="flex items-start justify-between gap-2">
                    <div class="min-w-0">
                      <div class="flex items-center gap-2">
                        <h3 class="truncate text-sm font-semibold text-slate-900">{{ speaker.name }}</h3>
                        <span class="rounded-full border border-brand-200 bg-brand-50 px-2 py-0.5 text-[10px] text-brand-700">{{ categoryLabel(speaker) }}</span>
                      </div>
                      <p class="mt-1 truncate text-xs text-stone-500">参考音频：{{ speaker.refAudioName || '未设置' }}</p>
                      <p v-if="speaker.refText" class="mt-0.5 line-clamp-2 text-xs text-slate-600">参考台词：{{ speaker.refText }}</p>
                    </div>
                    <div class="flex shrink-0 gap-1">
                      <button
                        class="inline-flex h-8 w-8 items-center justify-center rounded-lg text-stone-500 transition hover:bg-brand-50 hover:text-brand-700"
                        @click="openEditSpeaker(speaker)"
                      >
                        <PencilSquareIcon class="h-4 w-4" aria-hidden="true" />
                      </button>
                      <button
                        class="inline-flex h-8 w-8 items-center justify-center rounded-lg text-stone-500 transition hover:bg-rose-50 hover:text-rose-600"
                        @click="removeSpeaker(speaker)"
                      >
                        <TrashIcon class="h-4 w-4" aria-hidden="true" />
                      </button>
                    </div>
                  </div>
                </article>
              </div>

              <div v-else class="rounded-2xl border border-dashed border-brand-200 bg-white/85 p-4 text-sm text-stone-500">
                尚未配置说话人。点击「新增」添加一个语音克隆说话人。
              </div>
            </PanelCard>

            <PanelCard title="模型参数" subtitle="根据所选模型动态生成">
              <GenericTaskParamsForm :model-value="store.sessionConfig.modelParams" :task-config="activeTaskConfig" @update:model-value="onModelParamsChange" />
            </PanelCard>
          </div>
        </aside>
      </div>
    </Transition>
  </Teleport>

  <StreamingSpeakerForm :open="isFormOpen" :speaker="editingSpeaker" @close="closeSpeakerForm" @submit="submitSpeaker" />
</template>

<style scoped>
.streaming-drawer-enter-active,
.streaming-drawer-leave-active {
  transition: opacity 0.2s ease;
}
.streaming-drawer-enter-active aside,
.streaming-drawer-leave-active aside {
  transition: transform 0.2s ease;
}
.streaming-drawer-enter-from,
.streaming-drawer-leave-to {
  opacity: 0;
}
.streaming-drawer-enter-from aside,
.streaming-drawer-leave-to aside {
  transform: translateX(100%);
}
</style>
