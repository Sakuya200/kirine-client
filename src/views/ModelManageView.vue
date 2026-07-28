<script setup lang="ts">
import { ArrowDownTrayIcon, ArrowPathIcon, TrashIcon } from '@heroicons/vue/24/outline';
import { computed, onMounted, ref } from 'vue';

import BaseButton from '@/components/common/BaseButton.vue';
import BaseDialog from '@/components/common/BaseDialog.vue';
import BaseListbox from '@/components/common/BaseListbox.vue';
import BaseLoadingBanner from '@/components/common/BaseLoadingBanner.vue';
import BasePagination from '@/components/common/BasePagination.vue';
import PageHeader from '@/components/common/PageHeader.vue';
import PanelCard from '@/components/common/PanelCard.vue';
import { HISTORY_TASK_TYPE_TEXT, HistoryTaskType } from '@/enums/task';
import { MODEL_INSTALL_STATUS_STYLES, MODEL_INSTALL_STATUS_TEXT, ModelInstallStatus } from '@/enums/status';
import { HardwareType, HARDWARE_TYPE_TEXT } from '@/enums/settings';
import { usePollingResume } from '@/hooks/usePollingResume';
import { useModelStore } from '@/stores/models';
import type { ModelInfo } from '@/types/domain';

const modelStore = useModelStore();
const isMutating = ref(false);
const mutatingModelId = ref<number | null>(null);
const mutatingAction = ref<'install' | 'uninstall' | 'reinstall' | null>(null);
const uninstallTargetId = ref<number | null>(null);
// 正在持久化当前设备的模型 id：写入期间禁用该行下拉，避免并发覆盖。
const deviceUpdatingId = ref<number | null>(null);

// 模型目录数量有限，采用前端分页：loadModels 仍经 PageRequest 与 Rust 交互取全，
// 此处仅对已加载的 items 做切片展示，不破坏 modelStore getter（业务页依赖全量）。
const page = ref(1);
const pageSize = ref(10);
const pagedItems = computed(() => {
  const start = (page.value - 1) * pageSize.value;
  return modelStore.items.slice(start, start + pageSize.value);
});
const totalItems = computed(() => modelStore.items.length);

const onSetPage = (next: number) => {
  page.value = next;
};
const onSetPageSize = (next: number) => {
  pageSize.value = next;
  page.value = 1;
};

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
  [HistoryTaskType.VoiceDesign]: HISTORY_TASK_TYPE_TEXT[HistoryTaskType.VoiceDesign],
  [HistoryTaskType.StreamingSpeech]: HISTORY_TASK_TYPE_TEXT[HistoryTaskType.StreamingSpeech]
};

const refreshModels = async () => {
  await modelStore.loadModels();
};

const installStatusOf = (item: ModelInfo): ModelInstallStatus => modelStore.installStatusOf(item);

// 单设备模型只读展示（后端已自动回填 currentDevice）；多设备需用户主动选择。
const deviceSelectDisabled = (item: ModelInfo) => isMutating.value || deviceUpdatingId.value === item.id || item.supportedDevices.length <= 1;

const deviceOptions = (item: ModelInfo) =>
  item.supportedDevices.map(device => ({ label: HARDWARE_TYPE_TEXT[device] ?? device.toUpperCase(), value: device }));

const handleDeviceChange = async (item: ModelInfo, device: HardwareType) => {
  if (item.currentDevice === device) return;
  deviceUpdatingId.value = item.id;
  try {
    await modelStore.setCurrentDevice(item.id, device);
  } finally {
    deviceUpdatingId.value = null;
  }
};

const handleInstall = async (modelId: number) => {
  const target = modelStore.items.find(item => item.id === modelId);
  // 多设备模型未选设备时按钮已禁用；此处兜底，避免空设备进入安装。
  if (!target || target.currentDevice === null) {
    return;
  }
  const device = target.currentDevice;
  isMutating.value = true;
  mutatingModelId.value = modelId;
  mutatingAction.value = target.downloaded ? 'reinstall' : 'install';
  try {
    if (target.downloaded) {
      await modelStore.reinstallModel(modelId, device);
      return;
    }

    await modelStore.installModel(modelId, device);
  } finally {
    isMutating.value = false;
    mutatingModelId.value = null;
    mutatingAction.value = null;
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
  mutatingModelId.value = uninstallTarget.value.id;
  mutatingAction.value = 'uninstall';
  try {
    await modelStore.uninstallModel(uninstallTarget.value.id);
    closeUninstallDialog();
  } finally {
    isMutating.value = false;
    mutatingModelId.value = null;
    mutatingAction.value = null;
  }
};

// 解除锁屏 / 唤醒后重新同步模型状态：模型安装/卸载是阻塞式长任务，其 invoke
// 可能被系统睡眠冻结而卡住 isMutating；安装也可能在睡眠期间已完成。这里重拉
// 列表，并在后端状态已达到预期时复位 isMutating，避免加载条与按钮永久卡死。
usePollingResume(async () => {
  await modelStore.loadModels();

  if (isMutating.value && mutatingModelId.value !== null) {
    const target = modelStore.items.find(item => item.id === mutatingModelId.value) ?? null;
    const action = mutatingAction.value;
    const reached = target
      ? action === 'install' || action === 'reinstall'
        ? target.downloaded
        : action === 'uninstall'
          ? !target.downloaded
          : false
      : false;

    if (reached) {
      const reachedId = mutatingModelId.value;
      if (reachedId !== null) {
        modelStore.clearInstallFailed(reachedId);
      }
      isMutating.value = false;
      mutatingModelId.value = null;
      mutatingAction.value = null;
    }
  }
});

onMounted(async () => {
  await modelStore.ensureLoaded();
});
</script>

<template>
  <div class="space-y-5">
    <PageHeader title="模型管理" description="查看系统支持的基础模型、功能支持情况和当前安装状态，并执行安装或卸载。" eyebrow="Model Management" />

    <BaseLoadingBanner v-if="modelBusyLabel" :label="modelBusyLabel" :show-history-link="false" />

    <PanelCard title="模型列表" subtitle="系统中可用的基础模型及其支持的功能和安装状态" class="relative">
      <template #actions>
        <BaseButton tone="ghost" :loading="modelStore.isLoading" :disabled="isMutating" @click="refreshModels">
          <ArrowPathIcon v-if="!modelStore.isLoading" class="h-4 w-4" aria-hidden="true" />
          <span>{{ modelStore.isLoading ? '刷新中...' : '刷新列表' }}</span>
        </BaseButton>
      </template>

      <div v-if="modelStore.items.length > 0" class="overflow-x-auto">
        <table class="w-full min-w-[1080px] text-left text-sm">
          <thead>
            <tr class="border-b border-brand-100 text-xs uppercase tracking-wide text-stone-500">
              <th class="py-3 align-middle">模型</th>
              <th class="py-3 align-middle">版本</th>
              <th class="py-3 align-middle">支持功能</th>
              <th class="py-3 align-middle">依赖</th>
              <th class="py-3 align-middle">当前设备</th>
              <th class="py-3 align-middle">状态</th>
              <th class="py-3 align-middle">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="item in pagedItems" :key="item.id" class="border-b border-brand-50 text-slate-700 align-middle">
              <td class="py-3 align-middle font-medium text-slate-900">{{ item.modelName }}</td>
              <td class="py-3 align-middle">{{ item.modelVersion }}</td>
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
                <div class="w-44">
                  <BaseListbox
                    :model-value="item.currentDevice"
                    :options="deviceOptions(item)"
                    :disabled="deviceSelectDisabled(item)"
                    placeholder="请选择设备"
                    teleport
                    @update:model-value="handleDeviceChange(item, $event as HardwareType)"
                  />
                </div>
              </td>
              <td class="py-3 align-middle">
                <span class="rounded-full border px-2 py-1 text-[11px] font-medium" :class="MODEL_INSTALL_STATUS_STYLES[installStatusOf(item)]">
                  {{ MODEL_INSTALL_STATUS_TEXT[installStatusOf(item)] }}
                </span>
              </td>
              <td class="py-3 align-middle">
                <div class="flex flex-wrap items-center gap-2">
                  <BaseButton
                    tone="ghost"
                    size="sm"
                    :loading="(mutatingAction === 'install' || mutatingAction === 'reinstall') && mutatingModelId === item.id"
                    :disabled="isMutating || item.currentDevice === null"
                    :title="item.currentDevice === null ? '请先选择当前设备' : ''"
                    @click="handleInstall(item.id)"
                  >
                    <ArrowDownTrayIcon
                      v-if="!((mutatingAction === 'install' || mutatingAction === 'reinstall') && mutatingModelId === item.id)"
                      class="h-4 w-4"
                      aria-hidden="true"
                    />
                    <span>
                      {{
                        mutatingModelId === item.id && mutatingAction === 'reinstall'
                          ? '重装中...'
                          : mutatingModelId === item.id && mutatingAction === 'install'
                            ? '安装中...'
                            : item.downloaded
                              ? '重装'
                              : '安装'
                      }}
                    </span>
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

      <div v-if="modelStore.items.length > 0" class="mt-4">
        <BasePagination
          :current-page="page"
          :page-size="pageSize"
          :total-items="totalItems"
          :disabled="isMutating"
          :loading="modelStore.isLoading"
          @update:current-page="onSetPage"
          @update:page-size="onSetPageSize"
        />
      </div>
    </PanelCard>

    <BaseDialog :open="uninstallTarget !== null" title="卸载模型" @close="closeUninstallDialog">
      <p class="text-sm text-slate-600">
        <template v-if="uninstallTarget">
          将卸载模型“{{ uninstallTarget.modelName }} {{ uninstallTarget.modelVersion }}”的专属权重文件，并把状态改为未安装。共享依赖会保留。
        </template>
        <template v-else>未找到要卸载的模型。</template>
      </p>
      <template #footer>
        <BaseButton tone="ghost" @click="closeUninstallDialog">
          <span>取消</span>
        </BaseButton>
        <BaseButton tone="quiet" :loading="isMutating" :disabled="!uninstallTarget || isMutating" @click="confirmUninstall">
          <span>{{ isMutating ? '卸载中...' : '确认卸载' }}</span>
        </BaseButton>
      </template>
    </BaseDialog>
  </div>
</template>
