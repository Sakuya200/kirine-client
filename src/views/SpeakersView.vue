<script setup lang="ts">
import { ArrowPathIcon, EyeIcon, PencilSquareIcon, TrashIcon, XMarkIcon } from '@heroicons/vue/24/outline';
import { computed, onMounted, reactive, ref } from 'vue';

import BaseDialog from '@/components/common/BaseDialog.vue';
import BaseButton from '@/components/common/BaseButton.vue';
import BaseListbox from '@/components/common/BaseListbox.vue';
import PageHeader from '@/components/common/PageHeader.vue';
import PanelCard from '@/components/common/PanelCard.vue';
import { AppLanguage } from '@/enums/language';
import { SPEAKER_STATUS_STYLES, SPEAKER_STATUS_TEXT, SpeakerStatus } from '@/enums/status';
import { useSpeakerStore } from '@/stores/speakers';
import type { SpeakerProfile } from '@/types/domain';

type LanguageFilterValue = 'all' | AppLanguage;
type StatusFilterValue = 'all' | SpeakerStatus;

const speakerStore = useSpeakerStore();
const selectedSpeakerId = ref<number | null>(null);
const deleteTargetId = ref<number | null>(null);
const searchKeyword = ref('');

const languageOptions: Array<{ value: LanguageFilterValue; label: string }> = [
  { value: 'all', label: '全部语言' },
  { value: AppLanguage.Chinese, label: '中文' },
  { value: AppLanguage.English, label: '英文' },
  { value: AppLanguage.Japanese, label: '日文' }
];

const statusOptions: Array<{ value: StatusFilterValue; label: string }> = [
  { value: 'all', label: '全部状态' },
  { value: SpeakerStatus.Ready, label: SPEAKER_STATUS_TEXT[SpeakerStatus.Ready] },
  { value: SpeakerStatus.Training, label: SPEAKER_STATUS_TEXT[SpeakerStatus.Training] },
  { value: SpeakerStatus.Disabled, label: SPEAKER_STATUS_TEXT[SpeakerStatus.Disabled] }
];

const selectedLanguage = ref<LanguageFilterValue>(languageOptions[0].value);
const selectedStatus = ref<StatusFilterValue>(statusOptions[0].value);
const isEditDialogOpen = ref(false);
const isDeleteDialogOpen = ref(false);
const editForm = reactive({
  id: null as number | null,
  name: '',
  description: ''
});

const selectedSpeaker = computed(() => speakerStore.speakers.find(speaker => speaker.id === selectedSpeakerId.value) ?? null);
const deleteTarget = computed(() => speakerStore.speakers.find(speaker => speaker.id === deleteTargetId.value) ?? null);
const trimmedKeyword = computed(() => searchKeyword.value.trim().toLowerCase());
const canSaveSpeaker = computed(() => editForm.name.trim().length > 0 && editForm.description.trim().length > 0);

const filteredSpeakers = computed(() => {
  const keyword = trimmedKeyword.value;

  return speakerStore.speakers
    .filter(speaker => {
      const matchesKeyword =
        !keyword ||
        speaker.name.toLowerCase().includes(keyword) ||
        speaker.description.toLowerCase().includes(keyword) ||
        speakerStore.getLanguageLabel(speaker).toLowerCase().includes(keyword);
      const matchesLanguage = selectedLanguage.value === 'all' || speaker.languages.includes(selectedLanguage.value);
      const matchesStatus = selectedStatus.value === 'all' || speaker.status === selectedStatus.value;

      return matchesKeyword && matchesLanguage && matchesStatus;
    })
    .sort((left, right) => right.modifyTime.localeCompare(left.modifyTime));
});

const statusLabelMap: Record<SpeakerStatus, string> = {
  [SpeakerStatus.Ready]: SPEAKER_STATUS_TEXT[SpeakerStatus.Ready],
  [SpeakerStatus.Training]: SPEAKER_STATUS_TEXT[SpeakerStatus.Training],
  [SpeakerStatus.Disabled]: SPEAKER_STATUS_TEXT[SpeakerStatus.Disabled]
};

const statusClassMap: Record<SpeakerStatus, string> = {
  [SpeakerStatus.Ready]: SPEAKER_STATUS_STYLES[SpeakerStatus.Ready],
  [SpeakerStatus.Training]: SPEAKER_STATUS_STYLES[SpeakerStatus.Training],
  [SpeakerStatus.Disabled]: SPEAKER_STATUS_STYLES[SpeakerStatus.Disabled]
};

const openDetail = (speaker: SpeakerProfile) => {
  selectedSpeakerId.value = speaker.id;
};

const closeDetail = () => {
  selectedSpeakerId.value = null;
};

const openEdit = (speaker: SpeakerProfile) => {
  editForm.id = speaker.id;
  editForm.name = speaker.name;
  editForm.description = speaker.description;
  isEditDialogOpen.value = true;
};

const closeEditDialog = () => {
  isEditDialogOpen.value = false;
};

const saveSpeaker = async () => {
  if (!canSaveSpeaker.value) {
    return;
  }

  if (editForm.id === null) {
    return;
  }

  const updated = await speakerStore.updateSpeaker({
    id: editForm.id,
    name: editForm.name,
    description: editForm.description
  });

  if (updated) {
    closeEditDialog();
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
  const removed = await speakerStore.removeSpeaker(removedId);

  if (removed && selectedSpeakerId.value === removedId) {
    closeDetail();
  }

  if (removed) {
    closeDeleteDialog();
  }
};

onMounted(async () => {
  if (!speakerStore.initialized) {
    await speakerStore.loadSpeakers();
  }
});
</script>

<template>
  <div class="space-y-5">
    <PageHeader title="说话人管理" description="查看、筛选、编辑和删除已训练说话人，当前数据已接入本地数据库。" eyebrow="Speaker Management" />

    <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
      <article class="rounded-2xl border border-brand-200 bg-white/90 p-4">
        <p class="text-xs text-stone-500">说话人总数</p>
        <p class="mt-2 text-2xl font-semibold text-slate-900">{{ speakerStore.speakerCount }}</p>
      </article>
      <article class="rounded-2xl border border-emerald-200 bg-emerald-50/70 p-4">
        <p class="text-xs text-emerald-700">可用模型</p>
        <p class="mt-2 text-2xl font-semibold text-emerald-900">{{ speakerStore.readyCount }}</p>
      </article>
      <article class="rounded-2xl border border-amber-200 bg-amber-50/80 p-4">
        <p class="text-xs text-amber-700">训练中</p>
        <p class="mt-2 text-2xl font-semibold text-amber-900">{{ speakerStore.trainingCount }}</p>
      </article>
      <article class="rounded-2xl border border-brand-200 bg-brand-50/50 p-4">
        <p class="text-xs text-brand-700">样本总量</p>
        <p class="mt-2 text-2xl font-semibold text-brand-900">{{ speakerStore.totalSamples }}</p>
      </article>
    </div>

    <PanelCard title="说话人列表" subtitle="支持搜索、语言过滤、状态过滤、详情查看、编辑与删除操作">
      <template #actions>
        <BaseButton tone="ghost" :disabled="speakerStore.isLoading" @click="speakerStore.refreshSpeakers()">
          <ArrowPathIcon class="h-4 w-4" aria-hidden="true" />
          <span>{{ speakerStore.isLoading ? '刷新中...' : '刷新列表' }}</span>
        </BaseButton>
      </template>

      <div class="mb-4 grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)_minmax(0,1fr)]">
        <input
          v-model="searchKeyword"
          class="min-w-0 w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2 text-sm text-slate-700 sm:col-span-2 xl:col-span-1"
          placeholder="搜索名称、语言或备注"
        />
        <BaseListbox v-model="selectedLanguage" :options="languageOptions" />
        <BaseListbox v-model="selectedStatus" :options="statusOptions" />
      </div>

      <div v-if="filteredSpeakers.length > 0" class="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
        <article v-for="speaker in filteredSpeakers" :key="speaker.id" class="rounded-2xl border border-brand-200 bg-white/90 p-4">
          <div class="flex items-start justify-between gap-3">
            <h3 class="text-base font-semibold text-slate-900">{{ speaker.name }}</h3>
            <span class="rounded-full border px-2 py-1 text-[11px] font-medium" :class="statusClassMap[speaker.status]">
              {{ statusLabelMap[speaker.status] }}
            </span>
          </div>
          <p class="mt-1 text-xs text-stone-500">{{ speakerStore.getLanguageLabel(speaker) }} · 样本 {{ speaker.samples }} 条</p>
          <p class="mt-2 text-sm text-slate-600">{{ speaker.description }}</p>
          <p class="mt-3 text-xs text-stone-500">创建于 {{ speaker.createTime }} · 最近更新 {{ speaker.modifyTime }}</p>
          <div class="mt-3 flex flex-wrap gap-2">
            <BaseButton tone="ghost" @click="openDetail(speaker)">
              <EyeIcon class="h-4 w-4" aria-hidden="true" />
              <span>查看详情</span>
            </BaseButton>
            <BaseButton tone="quiet" @click="openEdit(speaker)">
              <PencilSquareIcon class="h-4 w-4" aria-hidden="true" />
              <span>编辑信息</span>
            </BaseButton>
            <BaseButton tone="quiet" @click="requestDelete(speaker)">
              <TrashIcon class="h-4 w-4" aria-hidden="true" />
              <span>删除</span>
            </BaseButton>
          </div>
        </article>
      </div>

      <div v-else class="rounded-2xl border border-dashed border-brand-200 bg-white/85 p-5 text-sm text-stone-500">
        {{ speakerStore.isLoading ? '正在加载说话人列表...' : '当前筛选条件下没有匹配的说话人。' }}
      </div>
    </PanelCard>

    <BaseDialog :open="selectedSpeaker !== null" title="说话人详情" @close="closeDetail">
      <div v-if="selectedSpeaker" class="space-y-2 text-sm text-slate-600">
        <p><span class="font-semibold text-slate-800">名称：</span>{{ selectedSpeaker.name }}</p>
        <p><span class="font-semibold text-slate-800">语言：</span>{{ speakerStore.getLanguageLabel(selectedSpeaker) }}</p>
        <p><span class="font-semibold text-slate-800">样本数：</span>{{ selectedSpeaker.samples }}</p>
        <p><span class="font-semibold text-slate-800">状态：</span>{{ statusLabelMap[selectedSpeaker.status] }}</p>
        <p><span class="font-semibold text-slate-800">创建时间：</span>{{ selectedSpeaker.createTime }}</p>
        <p><span class="font-semibold text-slate-800">更新时间：</span>{{ selectedSpeaker.modifyTime }}</p>
        <p><span class="font-semibold text-slate-800">备注：</span>{{ selectedSpeaker.description }}</p>
      </div>
      <template #footer>
        <BaseButton v-if="selectedSpeaker" tone="ghost" @click="openEdit(selectedSpeaker)">
          <PencilSquareIcon class="h-4 w-4" aria-hidden="true" />
          <span>编辑信息</span>
        </BaseButton>
        <BaseButton v-if="selectedSpeaker" tone="ghost" @click="requestDelete(selectedSpeaker)">
          <TrashIcon class="h-4 w-4" aria-hidden="true" />
          <span>删除</span>
        </BaseButton>
        <BaseButton tone="ghost" @click="closeDetail">
          <XMarkIcon class="h-4 w-4" aria-hidden="true" />
          <span>关闭</span>
        </BaseButton>
      </template>
    </BaseDialog>

    <BaseDialog :open="isEditDialogOpen" title="编辑说话人" @close="closeEditDialog">
      <div class="space-y-4">
        <label class="block text-sm text-slate-700">
          <span class="mb-1 block text-xs text-stone-500">名称</span>
          <input v-model="editForm.name" class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2" placeholder="请输入说话人名称" />
        </label>
        <label class="block text-sm text-slate-700">
          <span class="mb-1 block text-xs text-stone-500">备注</span>
          <textarea
            v-model="editForm.description"
            rows="4"
            class="w-full rounded-2xl border border-brand-200 bg-white/90 px-3 py-2"
            placeholder="请输入使用说明、适用场景或管理备注"
          />
        </label>
      </div>
      <template #footer>
        <BaseButton tone="ghost" @click="closeEditDialog">
          <XMarkIcon class="h-4 w-4" aria-hidden="true" />
          <span>取消</span>
        </BaseButton>
        <BaseButton :disabled="!canSaveSpeaker" @click="saveSpeaker">
          <PencilSquareIcon class="h-4 w-4" aria-hidden="true" />
          <span>保存修改</span>
        </BaseButton>
      </template>
    </BaseDialog>

    <BaseDialog :open="isDeleteDialogOpen" title="删除说话人" @close="closeDeleteDialog">
      <p class="text-sm text-slate-600">
        <template v-if="deleteTarget"> 将删除说话人“{{ deleteTarget.name }}”。该操作会执行逻辑删除，并从数据库查询结果中移除。 </template>
        <template v-else> 未找到要删除的说话人。 </template>
      </p>
      <template #footer>
        <BaseButton tone="ghost" @click="closeDeleteDialog">
          <XMarkIcon class="h-4 w-4" aria-hidden="true" />
          <span>取消</span>
        </BaseButton>
        <BaseButton tone="quiet" :disabled="!deleteTarget" @click="confirmDelete">
          <TrashIcon class="h-4 w-4" aria-hidden="true" />
          <span>确认删除</span>
        </BaseButton>
      </template>
    </BaseDialog>
  </div>
</template>
