<script setup lang="ts">
import { ArrowDownTrayIcon, ArrowPathIcon, EyeIcon, FolderOpenIcon, PencilSquareIcon, TrashIcon, XMarkIcon } from '@heroicons/vue/24/outline';
import { open } from '@tauri-apps/plugin-dialog';
import { convertFileSrc } from '@tauri-apps/api/core';
import { computed, onMounted, reactive, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';

import BaseDialog from '@/components/common/BaseDialog.vue';
import SpeakerAvatar from '@/components/speaker/SpeakerAvatar.vue';
import BaseButton from '@/components/common/BaseButton.vue';
import BaseListbox from '@/components/common/BaseListbox.vue';
import BasePagination from '@/components/common/BasePagination.vue';
import PageHeader from '@/components/common/PageHeader.vue';
import PanelCard from '@/components/common/PanelCard.vue';
import { SPEAKER_STATUS_STYLES, SPEAKER_STATUS_TEXT_KEY, SpeakerStatus } from '@/enums/status';
import { HistoryTaskType } from '@/enums/task';
import { useModels } from '@/hooks/useModels';
import { usePagination } from '@/hooks/usePagination';
import { normalizeSpeaker, useSpeakerStore } from '@/stores/speakers';
import { IMAGE_FILE_EXTENSIONS } from '@/types/streaming';
import type { SpeakerFilter, SpeakerPagedResult, SpeakerProfile } from '@/types/domain';

type StatusFilterValue = 'all' | SpeakerStatus;

const { t } = useI18n();

const speakerStore = useSpeakerStore();
const { getModelsByFeature, getModelVersionOptions, getModelLabel } = useModels();
const selectedSpeakerId = ref<number | null>(null);
const deleteTargetId = ref<number | null>(null);
const searchKeyword = ref('');

// 分页/筛选状态收归组件本地：不再与其他页面共享，避免跨页污染
const filter = ref<SpeakerFilter>({ keyword: null, status: null });
const stats = ref({ readyCount: 0, trainingCount: 0, disabledCount: 0, totalSamples: 0 });
const {
  items: speakers,
  total,
  page,
  pageSize,
  loading: isLoading,
  refresh,
  setPage,
  setPageSize,
  setFilter
} = usePagination<SpeakerProfile, SpeakerFilter>({
  command: 'list_speaker_infos',
  filter,
  initialPageSize: 9,
  normalize: normalizeSpeaker,
  errorLabel: t('common.store.speakers.loadFailed'),
  onResult: result => {
    const paged = result as SpeakerPagedResult;
    stats.value = {
      readyCount: paged.readyCount ?? 0,
      trainingCount: paged.trainingCount ?? 0,
      disabledCount: paged.disabledCount ?? 0,
      totalSamples: paged.totalSamples ?? 0
    };
  }
});

const statusOptions = computed<Array<{ value: StatusFilterValue; label: string }>>(() => [
  { value: 'all', label: t('common.allStatuses') },
  { value: SpeakerStatus.Ready, label: t(SPEAKER_STATUS_TEXT_KEY[SpeakerStatus.Ready]) },
  { value: SpeakerStatus.Training, label: t(SPEAKER_STATUS_TEXT_KEY[SpeakerStatus.Training]) },
  { value: SpeakerStatus.Disabled, label: t(SPEAKER_STATUS_TEXT_KEY[SpeakerStatus.Disabled]) }
]);

const selectedStatus = ref<StatusFilterValue>('all');
const isEditDialogOpen = ref(false);
const isImportDialogOpen = ref(false);
const isDeleteDialogOpen = ref(false);
const isSavingSpeaker = ref(false);
const isImportingSpeaker = ref(false);
const isDeletingSpeaker = ref(false);
const editForm = reactive({
  id: null as number | null,
  speakerName: '',
  description: '',
  // 头像编辑状态：avatarSourcePath 有值 = 用该文件覆盖；removeAvatar = 清除记录头像；
  // 两者均空 = 保持不变。existingAvatarContentType 仅用于预览回退。
  avatarSourcePath: null as string | null,
  avatarName: '',
  removeAvatar: false,
  existingAvatarContentType: null as string | null
});
const importForm = reactive({
  baseModel: '',
  modelVersion: '',
  sourceModelDirPath: '',
  speakerName: '',
  description: '',
  avatarSourcePath: null as string | null,
  avatarName: ''
});

const importableModelOptions = computed(() =>
  getModelsByFeature(HistoryTaskType.TextToSpeech).map(item => ({
    label: item.modelName,
    value: item.baseModel
  }))
);
const importModelVersionOptions = computed(() => getModelVersionOptions(importForm.baseModel));

const selectedSpeaker = computed(() => speakers.value.find(speaker => speaker.id === selectedSpeakerId.value) ?? null);
const deleteTarget = computed(() => speakers.value.find(speaker => speaker.id === deleteTargetId.value) ?? null);
const canSaveSpeaker = computed(() => editForm.speakerName.trim().length > 0 && editForm.description.trim().length > 0);
const canImportSpeaker = computed(
  () =>
    importForm.baseModel.trim().length > 0 &&
    importForm.modelVersion.trim().length > 0 &&
    importForm.sourceModelDirPath.trim().length > 0 &&
    importForm.speakerName.trim().length > 0 &&
    importForm.description.trim().length > 0
);

const onKeywordInput = () => {
  setFilter({ keyword: searchKeyword.value.trim() });
};

const onStatusChange = (value: StatusFilterValue) => {
  setFilter({ status: value === 'all' ? null : value });
};

const statusLabelMap = computed<Record<SpeakerStatus, string>>(() => ({
  [SpeakerStatus.Ready]: t(SPEAKER_STATUS_TEXT_KEY[SpeakerStatus.Ready]),
  [SpeakerStatus.Training]: t(SPEAKER_STATUS_TEXT_KEY[SpeakerStatus.Training]),
  [SpeakerStatus.Disabled]: t(SPEAKER_STATUS_TEXT_KEY[SpeakerStatus.Disabled])
}));

const statusClassMap: Record<SpeakerStatus, string> = {
  [SpeakerStatus.Ready]: SPEAKER_STATUS_STYLES[SpeakerStatus.Ready],
  [SpeakerStatus.Training]: SPEAKER_STATUS_STYLES[SpeakerStatus.Training],
  [SpeakerStatus.Disabled]: SPEAKER_STATUS_STYLES[SpeakerStatus.Disabled]
};

const getSpeakerModelLabel = (speaker: SpeakerProfile) => getModelLabel(speaker.baseModel);

const openDetail = (speaker: SpeakerProfile) => {
  selectedSpeakerId.value = speaker.id;
};

const closeDetail = () => {
  selectedSpeakerId.value = null;
};

const openEdit = (speaker: SpeakerProfile) => {
  editForm.id = speaker.id;
  editForm.speakerName = speaker.speakerName;
  editForm.description = speaker.description;
  editForm.avatarSourcePath = null;
  editForm.avatarName = '';
  editForm.removeAvatar = false;
  editForm.existingAvatarContentType = speaker.avatarContentType;
  isEditDialogOpen.value = true;
};

const selectAvatarFile = async () => {
  const selected = await open({
    multiple: false,
    directory: false,
    title: t('speakers.editDialog.selectAvatar'),
    filters: [{ name: t('speakers.editDialog.imageFiles'), extensions: [...IMAGE_FILE_EXTENSIONS] }]
  });
  if (typeof selected === 'string' && selected.trim().length > 0) {
    const segments = selected.split(/[/\\]/);
    const fileName = segments[segments.length - 1] ?? selected;
    // 两个对话框互斥打开，按当前打开的对话框写入对应表单
    if (isEditDialogOpen.value) {
      editForm.avatarSourcePath = selected;
      editForm.removeAvatar = false;
      editForm.avatarName = fileName;
    } else if (isImportDialogOpen.value) {
      importForm.avatarSourcePath = selected;
      importForm.avatarName = fileName;
    }
  }
};

const clearEditAvatarSelection = () => {
  editForm.avatarSourcePath = null;
  editForm.avatarName = '';
};

const removeEditAvatar = () => {
  editForm.avatarSourcePath = null;
  editForm.avatarName = '';
  editForm.removeAvatar = true;
};

const clearImportAvatar = () => {
  importForm.avatarSourcePath = null;
  importForm.avatarName = '';
};

// 编辑对话框头像预览：新选文件走 asset 协议直显；记录头像走 SpeakerAvatar；移除/无头像走占位
const editAvatarPreviewUrl = computed(() =>
  editForm.avatarSourcePath ? convertFileSrc(editForm.avatarSourcePath) : null
);
const editAvatarShowRecord = computed(
  () => !editForm.avatarSourcePath && !editForm.removeAvatar && Boolean(editForm.existingAvatarContentType)
);
const editAvatarHasDisplay = computed(
  () => Boolean(editForm.avatarSourcePath) || editAvatarShowRecord.value
);

const closeEditDialog = () => {
  isEditDialogOpen.value = false;
};

const resetImportForm = () => {
  importForm.baseModel = String(importableModelOptions.value[0]?.value ?? '');
  importForm.modelVersion = String(importModelVersionOptions.value[0]?.value ?? '');
  importForm.sourceModelDirPath = '';
  importForm.speakerName = '';
  importForm.description = '';
  importForm.avatarSourcePath = null;
  importForm.avatarName = '';
};

const openImportDialog = () => {
  resetImportForm();
  isImportDialogOpen.value = true;
};

const closeImportDialog = () => {
  isImportDialogOpen.value = false;
};

const pickImportModelDirectory = async () => {
  const selected = await open({
    directory: true,
    multiple: false,
    title: t('speakers.importDialog.dirPickerTitle')
  });

  if (typeof selected === 'string') {
    importForm.sourceModelDirPath = selected;
  }
};

const submitImportSpeaker = async () => {
  if (!canImportSpeaker.value) {
    return;
  }

  isImportingSpeaker.value = true;
  const imported = await speakerStore.importSpeaker({
    baseModel: importForm.baseModel,
    modelVersion: importForm.modelVersion,
    sourceModelDirPath: importForm.sourceModelDirPath.trim(),
    speakerName: importForm.speakerName.trim(),
    description: importForm.description.trim(),
    avatarSourcePath: importForm.avatarSourcePath
  });
  isImportingSpeaker.value = false;

  if (imported) {
    closeImportDialog();
    await refresh();
  }
};

const saveSpeaker = async () => {
  if (!canSaveSpeaker.value) {
    return;
  }

  if (editForm.id === null) {
    return;
  }

  isSavingSpeaker.value = true;
  const updated = await speakerStore.updateSpeaker({
    id: editForm.id,
    speakerName: editForm.speakerName,
    description: editForm.description,
    avatarSourcePath: editForm.avatarSourcePath,
    removeAvatar: editForm.removeAvatar
  });
  isSavingSpeaker.value = false;

  if (updated) {
    closeEditDialog();
    await refresh();
  }
};

const requestDelete = (speaker: SpeakerProfile) => {
  deleteTargetId.value = speaker.id;
  isDeleteDialogOpen.value = true;
};

const closeDeleteDialog = () => {
  isDeleteDialogOpen.value = false;
  deleteTargetId.value = null;
};

const confirmDelete = async () => {
  if (!deleteTarget.value) {
    return;
  }

  const removedId = deleteTarget.value.id;
  isDeletingSpeaker.value = true;
  const removed = await speakerStore.removeSpeaker(removedId, deleteTarget.value.speakerName);
  isDeletingSpeaker.value = false;

  if (removed && selectedSpeakerId.value === removedId) {
    closeDetail();
  }

  if (removed) {
    closeDeleteDialog();
    await refresh();
  }
};

watch(
  importableModelOptions,
  options => {
    if (options.length === 0) {
      importForm.baseModel = '';
      return;
    }

    if (!options.some(option => option.value === importForm.baseModel)) {
      importForm.baseModel = String(options[0]?.value ?? '');
    }
  },
  { immediate: true }
);

watch(
  importModelVersionOptions,
  options => {
    if (options.length === 0) {
      importForm.modelVersion = '';
      return;
    }

    if (!options.some(option => option.value === importForm.modelVersion)) {
      importForm.modelVersion = String(options[0]?.value ?? '');
    }
  },
  { immediate: true }
);

onMounted(() => {
  void refresh();
});
</script>

<template>
  <div class="space-y-5">
    <PageHeader :title="t('speakers.title')" :description="t('speakers.description')" eyebrow="Speaker Management" />

    <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
      <article class="rounded-2xl border border-brand-200 bg-white/90 p-4">
        <p class="text-xs text-stone-500">{{ t('speakers.stats.total') }}</p>
        <p class="mt-2 text-2xl font-semibold text-slate-900">{{ total }}</p>
      </article>
      <article class="rounded-2xl border border-emerald-200 bg-emerald-50/70 p-4">
        <p class="text-xs text-emerald-700">{{ t('speakers.stats.readyModels') }}</p>
        <p class="mt-2 text-2xl font-semibold text-emerald-900">{{ stats.readyCount }}</p>
      </article>
      <article class="rounded-2xl border border-amber-200 bg-amber-50/80 p-4">
        <p class="text-xs text-amber-700">{{ t('speakers.stats.training') }}</p>
        <p class="mt-2 text-2xl font-semibold text-amber-900">{{ stats.trainingCount }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-brand-50/50 p-4">
        <p class="text-xs text-brand-700">{{ t('speakers.stats.totalSamples') }}</p>
        <p class="mt-2 text-2xl font-semibold text-brand-900">{{ stats.totalSamples }}</p>
      </article>
    </div>

    <PanelCard :title="t('speakers.panel.title')" :subtitle="t('speakers.panel.subtitle')">
      <template #actions>
        <div class="flex flex-wrap gap-2">
          <BaseButton tone="ghost" :disabled="importableModelOptions.length === 0" @click="openImportDialog">
            <ArrowDownTrayIcon class="h-4 w-4" aria-hidden="true" />
            <span>{{ t('speakers.panel.importModel') }}</span>
          </BaseButton>
          <BaseButton tone="ghost" :loading="isLoading" @click="refresh()">
            <ArrowPathIcon v-if="!isLoading" class="h-4 w-4" aria-hidden="true" />
            <span>{{ isLoading ? t('speakers.panel.refreshing') : t('speakers.panel.refresh') }}</span>
          </BaseButton>
        </div>
      </template>

      <div class="mb-4 grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)_minmax(0,1fr)]">
        <input
          v-model="searchKeyword"
          class="min-w-0 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700 sm:col-span-2 xl:col-span-1"
          :placeholder="t('speakers.panel.searchPlaceholder')"
          @input="onKeywordInput"
        />
        <BaseListbox :model-value="selectedStatus" :options="statusOptions" @update:model-value="onStatusChange($event as StatusFilterValue)" />
      </div>

      <div v-if="speakers.length > 0" class="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
        <article v-for="speaker in speakers" :key="speaker.id" class="rounded-2xl border border-brand-200 bg-white/90 p-4">
          <div class="flex items-start justify-between gap-3">
            <div class="flex min-w-0 flex-1 items-center gap-3">
              <SpeakerAvatar :speaker-id="speaker.id" :has-avatar="speaker.avatarContentType !== null" :speaker-name="speaker.speakerName" size-class="h-12 w-12" />
              <h3 class="truncate text-base font-semibold text-slate-900">{{ speaker.speakerName }}</h3>
            </div>
            <div class="flex shrink-0 flex-wrap justify-end gap-2">
              <span class="rounded-full border border-sky-200 bg-sky-50 px-2 py-1 text-[11px] font-medium text-sky-700">
                {{ getSpeakerModelLabel(speaker) }}
              </span>
              <span class="rounded-full border px-2 py-1 text-[11px] font-medium" :class="statusClassMap[speaker.status]">
                {{ statusLabelMap[speaker.status] }}
              </span>
            </div>
          </div>
          <p class="mt-1 text-xs text-stone-500">{{ t('speakers.panel.samples', { count: speaker.samples }) }}</p>
          <p class="mt-2 text-sm text-slate-600">{{ speaker.description }}</p>
          <p class="mt-3 text-xs text-stone-500">{{ t('speakers.panel.createdAt', { time: speaker.createTime, modifyTime: speaker.modifyTime }) }}</p>
          <div class="mt-3 flex flex-wrap gap-2">
            <BaseButton tone="ghost" @click="openDetail(speaker)">
              <EyeIcon class="h-4 w-4" aria-hidden="true" />
              <span>{{ t('speakers.panel.viewDetail') }}</span>
            </BaseButton>
            <BaseButton tone="quiet" @click="openEdit(speaker)">
              <PencilSquareIcon class="h-4 w-4" aria-hidden="true" />
              <span>{{ t('speakers.panel.edit') }}</span>
            </BaseButton>
            <BaseButton tone="quiet" @click="requestDelete(speaker)">
              <TrashIcon class="h-4 w-4" aria-hidden="true" />
              <span>{{ t('speakers.panel.delete') }}</span>
            </BaseButton>
          </div>
        </article>
      </div>

      <div v-else class="rounded-2xl border border-dashed border-brand-200 bg-white/85 p-5 text-sm text-stone-500">
        {{ isLoading ? t('speakers.panel.loading') : t('speakers.panel.empty') }}
      </div>

      <div v-if="speakers.length > 0" class="mt-4">
        <BasePagination
          :current-page="page"
          :page-size="pageSize"
          :page-size-options="[9, 18, 27]"
          :total-items="total"
          :disabled="isLoading"
          :loading="isLoading"
          @update:current-page="setPage"
          @update:page-size="setPageSize"
        />
      </div>
    </PanelCard>

    <BaseDialog :open="selectedSpeaker !== null" :title="t('speakers.detail.title')" @close="closeDetail">
      <div v-if="selectedSpeaker" class="space-y-2 text-sm text-slate-600">
        <div class="flex items-center gap-3">
          <SpeakerAvatar :speaker-id="selectedSpeaker.id" :has-avatar="selectedSpeaker.avatarContentType !== null" :speaker-name="selectedSpeaker.speakerName" size-class="h-12 w-12" />
          <span class="font-semibold text-slate-800">{{ selectedSpeaker.speakerName }}</span>
        </div>
        <p><span class="font-semibold text-slate-800">{{ t('speakers.detail.name') }}</span>{{ selectedSpeaker.speakerName }}</p>
        <p><span class="font-semibold text-slate-800">{{ t('speakers.detail.model') }}</span>{{ getSpeakerModelLabel(selectedSpeaker) }}</p>
        <p><span class="font-semibold text-slate-800">{{ t('speakers.detail.samples') }}</span>{{ selectedSpeaker.samples }}</p>
        <p><span class="font-semibold text-slate-800">{{ t('speakers.detail.status') }}</span>{{ statusLabelMap[selectedSpeaker.status] }}</p>
        <p><span class="font-semibold text-slate-800">{{ t('speakers.detail.createTime') }}</span>{{ selectedSpeaker.createTime }}</p>
        <p><span class="font-semibold text-slate-800">{{ t('speakers.detail.modifyTime') }}</span>{{ selectedSpeaker.modifyTime }}</p>
        <p><span class="font-semibold text-slate-800">{{ t('speakers.detail.description') }}</span>{{ selectedSpeaker.description }}</p>
      </div>
      <template #footer>
        <BaseButton v-if="selectedSpeaker" tone="ghost" @click="openEdit(selectedSpeaker)">
          <PencilSquareIcon class="h-4 w-4" aria-hidden="true" />
          <span>{{ t('speakers.panel.edit') }}</span>
        </BaseButton>
        <BaseButton v-if="selectedSpeaker" tone="ghost" @click="requestDelete(selectedSpeaker)">
          <TrashIcon class="h-4 w-4" aria-hidden="true" />
          <span>{{ t('speakers.panel.delete') }}</span>
        </BaseButton>
        <BaseButton tone="ghost" @click="closeDetail">
          <XMarkIcon class="h-4 w-4" aria-hidden="true" />
          <span>{{ t('speakers.detail.close') }}</span>
        </BaseButton>
      </template>
    </BaseDialog>

    <BaseDialog :open="isEditDialogOpen" :title="t('speakers.editDialog.title')" @close="closeEditDialog">
      <div class="space-y-4">
        <div class="text-sm text-slate-700">
          <span class="mb-1 block text-xs text-stone-500">{{ t('speakers.editDialog.avatar') }}</span>
          <div class="flex items-center gap-3 rounded-2xl border border-brand-200 bg-white/90 px-3 py-2">
            <SpeakerAvatar
              v-if="editAvatarShowRecord"
              :speaker-id="editForm.id ?? 0"
              :has-avatar="true"
              :speaker-name="editForm.speakerName"
              size-class="h-12 w-12"
            />
            <img
              v-else-if="editAvatarPreviewUrl"
              :src="editAvatarPreviewUrl"
              class="h-12 w-12 shrink-0 rounded-full border border-brand-200 object-cover shadow-soft"
              alt=""
            />
            <div
              v-else
              class="flex h-12 w-12 shrink-0 items-center justify-center rounded-full border border-brand-200 bg-brand-100 text-sm font-semibold text-brand-700"
            >
              {{ Array.from(editForm.speakerName.trim() || '？')[0]?.toUpperCase() ?? '？' }}
            </div>
            <div class="min-w-0 flex-1 break-all text-sm text-slate-700">
              {{ editForm.avatarName || (editAvatarShowRecord ? t('speakers.editDialog.recordAvatar') : t('speakers.editDialog.noAvatar')) }}
            </div>
            <BaseButton tone="ghost" @click="selectAvatarFile">
              <FolderOpenIcon class="h-4 w-4" aria-hidden="true" />
              <span>{{ t('speakers.editDialog.selectImage') }}</span>
            </BaseButton>
            <BaseButton v-if="editAvatarPreviewUrl" tone="ghost" @click="clearEditAvatarSelection">
              <span>{{ t('speakers.editDialog.clearSelection') }}</span>
            </BaseButton>
            <BaseButton v-if="!editAvatarPreviewUrl && !editForm.removeAvatar && (editAvatarHasDisplay || editForm.existingAvatarContentType)" tone="ghost" @click="removeEditAvatar">
              <span>{{ t('speakers.editDialog.removeAvatar') }}</span>
            </BaseButton>
            <BaseButton v-if="editForm.removeAvatar" tone="ghost" @click="editForm.removeAvatar = false">
              <span>{{ t('speakers.editDialog.undoRemove') }}</span>
            </BaseButton>
          </div>
        </div>
        <label class="block text-sm text-slate-700">
          <span class="mb-1 block text-xs text-stone-500">{{ t('speakers.editDialog.name') }}</span>
          <input v-model="editForm.speakerName" class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2" :placeholder="t('speakers.editDialog.namePlaceholder')" />
        </label>
        <label class="block text-sm text-slate-700">
          <span class="mb-1 block text-xs text-stone-500">{{ t('speakers.editDialog.description') }}</span>
          <textarea
            v-model="editForm.description"
            rows="4"
            class="w-full rounded-2xl border border-brand-200 bg-white/90 px-3 py-2"
            :placeholder="t('speakers.editDialog.descriptionPlaceholder')"
          />
        </label>
      </div>
      <template #footer>
        <BaseButton tone="ghost" @click="closeEditDialog">
          <XMarkIcon class="h-4 w-4" aria-hidden="true" />
          <span>{{ t('common.cancel') }}</span>
        </BaseButton>
        <BaseButton :loading="isSavingSpeaker" :disabled="!canSaveSpeaker" @click="saveSpeaker">
          <PencilSquareIcon v-if="!isSavingSpeaker" class="h-4 w-4" aria-hidden="true" />
          <span>{{ isSavingSpeaker ? t('common.saving') : t('speakers.editDialog.save') }}</span>
        </BaseButton>
      </template>
    </BaseDialog>

    <BaseDialog :open="isImportDialogOpen" :title="t('speakers.importDialog.title')" @close="closeImportDialog">
      <div class="space-y-4">
        <label class="block text-sm text-slate-700">
          <span class="mb-1 block text-xs text-stone-500">{{ t('speakers.importDialog.modelType') }}</span>
          <BaseListbox v-model="importForm.baseModel" :options="importableModelOptions" />
        </label>
        <label class="block text-sm text-slate-700">
          <span class="mb-1 block text-xs text-stone-500">{{ t('tts.form.modelVersion') }}</span>
          <BaseListbox v-model="importForm.modelVersion" :options="importModelVersionOptions" />
        </label>
        <label class="block text-sm text-slate-700">
          <span class="mb-1 block text-xs text-stone-500">{{ t('speakers.importDialog.modelDir') }}</span>
          <div class="flex gap-2">
            <input
              v-model="importForm.sourceModelDirPath"
              class="min-w-0 flex-1 rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
              :placeholder="t('speakers.importDialog.dirPlaceholder')"
              readonly
            />
            <BaseButton tone="ghost" @click="pickImportModelDirectory">
              <FolderOpenIcon class="h-4 w-4" aria-hidden="true" />
              <span>{{ t('speakers.importDialog.selectDir') }}</span>
            </BaseButton>
          </div>
        </label>
        <label class="block text-sm text-slate-700">
          <span class="mb-1 block text-xs text-stone-500">{{ t('speakers.importDialog.speakerName') }}</span>
          <input v-model="importForm.speakerName" class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2" :placeholder="t('speakers.editDialog.namePlaceholder')" />
        </label>
        <label class="block text-sm text-slate-700">
          <span class="mb-1 block text-xs text-stone-500">{{ t('speakers.importDialog.speakerDescription') }}</span>
          <textarea
            v-model="importForm.description"
            rows="4"
            class="w-full rounded-2xl border border-brand-200 bg-white/90 px-3 py-2"
            :placeholder="t('speakers.importDialog.speakerDescriptionPlaceholder')"
          />
        </label>
        <div class="text-sm text-slate-700">
          <span class="mb-1 block text-xs text-stone-500">{{ t('speakers.importDialog.avatarOptional') }}</span>
          <div class="flex items-center gap-3 rounded-2xl border border-brand-200 bg-white/90 px-3 py-2">
            <img
              v-if="importForm.avatarSourcePath"
              :src="convertFileSrc(importForm.avatarSourcePath)"
              class="h-12 w-12 shrink-0 rounded-full border border-brand-200 object-cover shadow-soft"
              alt=""
            />
            <span class="min-w-0 flex-1 break-all text-sm text-slate-700">
              {{ importForm.avatarName || t('speakers.editDialog.noAvatar') }}
            </span>
            <BaseButton tone="ghost" @click="selectAvatarFile">
              <FolderOpenIcon class="h-4 w-4" aria-hidden="true" />
              <span>{{ t('speakers.editDialog.selectImage') }}</span>
            </BaseButton>
            <BaseButton v-if="importForm.avatarSourcePath" tone="ghost" @click="clearImportAvatar">
              <span>{{ t('speakers.editDialog.clearSelection') }}</span>
            </BaseButton>
          </div>
        </div>
      </div>
      <template #footer>
        <BaseButton tone="ghost" @click="closeImportDialog">
          <XMarkIcon class="h-4 w-4" aria-hidden="true" />
          <span>{{ t('common.cancel') }}</span>
        </BaseButton>
        <BaseButton :loading="isImportingSpeaker" :disabled="!canImportSpeaker" @click="submitImportSpeaker">
          <ArrowDownTrayIcon v-if="!isImportingSpeaker" class="h-4 w-4" aria-hidden="true" />
          <span>{{ isImportingSpeaker ? t('speakers.importDialog.importing') : t('speakers.importDialog.confirm') }}</span>
        </BaseButton>
      </template>
    </BaseDialog>

    <BaseDialog :open="isDeleteDialogOpen" :title="t('speakers.deleteDialog.title')" @close="closeDeleteDialog">
      <p class="text-sm text-slate-600">
        <template v-if="deleteTarget"> {{ t('speakers.deleteDialog.confirm', { name: deleteTarget.speakerName }) }} </template>
        <template v-else> {{ t('speakers.deleteDialog.notFound') }} </template>
      </p>
      <template #footer>
        <BaseButton tone="ghost" @click="closeDeleteDialog">
          <XMarkIcon class="h-4 w-4" aria-hidden="true" />
          <span>{{ t('common.cancel') }}</span>
        </BaseButton>
        <BaseButton tone="quiet" :loading="isDeletingSpeaker" :disabled="!deleteTarget || isDeletingSpeaker" @click="confirmDelete">
          <TrashIcon v-if="!isDeletingSpeaker" class="h-4 w-4" aria-hidden="true" />
          <span>{{ isDeletingSpeaker ? t('speakers.deleteDialog.deleting') : t('speakers.deleteDialog.confirmDelete') }}</span>
        </BaseButton>
      </template>
    </BaseDialog>
  </div>
</template>
