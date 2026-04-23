<script setup lang="ts">
import { ArrowDownTrayIcon, ArrowPathIcon, TrashIcon } from '@heroicons/vue/24/outline';
import { computed, onMounted, ref } from 'vue';

import BaseButton from '@/components/common/BaseButton.vue';
import BaseDialog from '@/components/common/BaseDialog.vue';
import BaseLoadingBanner from '@/components/common/BaseLoadingBanner.vue';
import BaseLoadingIndicator from '@/components/common/BaseLoadingIndicator.vue';
import PageHeader from '@/components/common/PageHeader.vue';
import PanelCard from '@/components/common/PanelCard.vue';
import { HISTORY_TASK_TYPE_TEXT, HistoryTaskType } from '@/enums/task';
import { useModelStore } from '@/stores/models';

const modelStore = useModelStore();
const isMutating = ref(false);
const uninstallTargetId = ref<number | null>(null);

const uninstallTarget = computed(() => modelStore.items.find(item => item.id === uninstallTargetId.value) ?? null);
const modelBusyLabel = computed(() => {
  if (isMutating.value) {
    return '正在处理模型安装或卸载，请稍候';
  }

  if (modelStore.isLoading) {
    return '正在加载模型列表';
  }

  return '';
});

const featureLabelMap: Record<string, string> = {
  [HistoryTaskType.TextToSpeech]: HISTORY_TASK_TYPE_TEXT[HistoryTaskType.TextToSpeech],
  [HistoryTaskType.VoiceClone]: HISTORY_TASK_TYPE_TEXT[HistoryTaskType.VoiceClone],
  [HistoryTaskType.ModelTraining]: HISTORY_TASK_TYPE_TEXT[HistoryTaskType.ModelTraining],
  lora: 'LoRA'
};

const refreshModels = async () => {
  await modelStore.loadModels();
};

const handleInstall = async (modelId: number) => {
  isMutating.value = true;
  try {
    await modelStore.installModel(modelId);
  } finally {
    isMutating.value = false;
  }
};

const requestUninstall = (modelId: number) => {
  uninstallTargetId.value = modelId;
};

const closeUninstallDialog = () => {
  uninstallTargetId.value = null;
};

const confirmUninstall = async () => {
  if (!uninstallTarget.value) {
    return;
  }

  isMutating.value = true;
  try {
    await modelStore.uninstallModel(uninstallTarget.value.id);
    closeUninstallDialog();
  } finally {
    isMutating.value = false;
  }
};

onMounted(async () => {
  await modelStore.ensureLoaded();
});
</script>

<template>
  <div class="space-y-5">
    <PageHeader title="模型管理" description="查看系统支持的基础模型、功能支持情况和当前安装状态，并执行安装或卸载。" eyebrow="Model Management" />

    <BaseLoadingBanner v-if="modelBusyLabel" :label="modelBusyLabel" />

    <PanelCard title="模型列表" subtitle="状态来自本地数据库中的 model_info.downloaded 字段。">
      <template #actions>
        <BaseButton tone="ghost" :disabled="modelStore.isLoading || isMutating" @click="refreshModels">
          <BaseLoadingIndicator v-if="modelStore.isLoading" size="sm" tone="muted" />
          <ArrowPathIcon v-else class="h-4 w-4" aria-hidden="true" />
          <span>{{ modelStore.isLoading ? '刷新中...' : '刷新列表' }}</span>
        </BaseButton>
      </template>

      <div v-if="modelStore.items.length > 0" class="overflow-x-auto">
        <table class="w-full min-w-[920px] text-left text-sm">
          <thead>
            <tr class="border-b border-brand-100 text-xs uppercase tracking-wide text-stone-500">
              <th class="py-3 align-middle">模型</th>
              <th class="py-3 align-middle">规模</th>
              <th class="py-3 align-middle">支持功能</th>
              <th class="py-3 align-middle">依赖</th>
              <th class="py-3 align-middle">状态</th>
              <th class="py-3 align-middle">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="item in modelStore.items" :key="item.id" class="border-b border-brand-50 text-slate-700 align-middle">
              <td class="py-3 align-middle font-medium text-slate-900">{{ item.modelName }}</td>
              <td class="py-3 align-middle">{{ item.modelScale }}</td>
              <td class="py-3 align-middle">
                <div class="flex flex-wrap gap-1.5">
                  <span
                    v-for="feature in item.supportedFeatureList"
                    :key="feature"
                    class="rounded-full border border-brand-200 bg-brand-50 px-2 py-1 text-[11px] text-brand-700"
                  >
                    {{ featureLabelMap[feature] ?? feature }}
                  </span>
                </div>
              </td>
              <td class="py-3 align-middle text-xs text-stone-500">
                <div v-for="name in item.requiredModelNameList" :key="name">{{ name }}</div>
              </td>
              <td class="py-3 align-middle">
                <span
                  class="rounded-full border px-2 py-1 text-[11px] font-medium"
                  :class="item.downloaded ? 'border-emerald-200 bg-emerald-50 text-emerald-700' : 'border-stone-200 bg-stone-100 text-stone-600'"
                >
                  {{ item.downloaded ? '已安装' : '未安装' }}
                </span>
              </td>
              <td class="py-3 align-middle">
                <div class="flex flex-wrap items-center gap-2">
                  <BaseButton tone="ghost" size="sm" :disabled="isMutating" @click="handleInstall(item.id)">
                    <ArrowDownTrayIcon class="h-4 w-4" aria-hidden="true" />
                    <span>{{ item.downloaded ? '重装' : '安装' }}</span>
                  </BaseButton>
                  <BaseButton tone="quiet" size="sm" :disabled="isMutating || !item.downloaded" @click="requestUninstall(item.id)">
                    <TrashIcon class="h-4 w-4" aria-hidden="true" />
                    <span>卸载</span>
                  </BaseButton>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div v-else class="rounded-2xl border border-dashed border-brand-200 bg-white/85 p-5 text-sm text-stone-500">
        {{ modelStore.isLoading ? '正在加载模型列表...' : '当前没有可展示的模型信息。' }}
      </div>
    </PanelCard>

    <BaseDialog :open="uninstallTarget !== null" title="卸载模型" @close="closeUninstallDialog">
      <p class="text-sm text-slate-600">
        <template v-if="uninstallTarget">
          将卸载模型“{{ uninstallTarget.modelName }} {{ uninstallTarget.modelScale }}”的专属权重文件，并把状态改为未安装。共享依赖会保留。
        </template>
        <template v-else>未找到要卸载的模型。</template>
      </p>
      <template #footer>
        <BaseButton tone="ghost" @click="closeUninstallDialog">
          <span>取消</span>
        </BaseButton>
        <BaseButton tone="quiet" :disabled="!uninstallTarget || isMutating" @click="confirmUninstall">
          <BaseLoadingIndicator v-if="isMutating" size="sm" tone="muted" />
          <span>{{ isMutating ? '卸载中...' : '确认卸载' }}</span>
        </BaseButton>
      </template>
    </BaseDialog>
  </div>
</template>
