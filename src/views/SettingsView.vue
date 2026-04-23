<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { CheckBadgeIcon, FolderIcon } from '@heroicons/vue/24/outline';
import { Tab, TabGroup, TabList, TabPanel, TabPanels } from '@headlessui/vue';
import { computed, onMounted, reactive, ref } from 'vue';

import BaseButton from '@/components/common/BaseButton.vue';
import BaseLoadingBanner from '@/components/common/BaseLoadingBanner.vue';
import BaseLoadingIndicator from '@/components/common/BaseLoadingIndicator.vue';
import BaseListbox from '@/components/common/BaseListbox.vue';
import PageHeader from '@/components/common/PageHeader.vue';
import PanelCard from '@/components/common/PanelCard.vue';
import { ATTENTION_IMPLEMENTATION_TEXT, AttentionImplementation, HARDWARE_TYPE_TEXT, HardwareType } from '@/enums/settings';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { useUiStore } from '@/stores/ui';

const settingTabs = ['连接配置', '模型资源', '缓存配置'];

interface SettingsForm {
  apiUrl: string;
  apiToken: string;
  modelDir: string;
  dataDir: string;
  logCacheDir: string;
  hardwareType: HardwareType;
  attnImplementation: AttentionImplementation;
}

interface SettingsResponse extends SettingsForm {
  restartRequired: boolean;
  migratedDirectories: string[];
  removableDirectories: string[];
}

const DEFAULT_SETTINGS_FORM: SettingsForm = {
  apiUrl: '',
  apiToken: '',
  modelDir: '',
  dataDir: '',
  logCacheDir: '',
  hardwareType: HardwareType.Cpu,
  attnImplementation: AttentionImplementation.Sdpa
};

const form = reactive<SettingsForm>({ ...DEFAULT_SETTINGS_FORM });
const attnImplementationOptions = Object.values(AttentionImplementation).map(value => ({
  label: ATTENTION_IMPLEMENTATION_TEXT[value],
  value
}));
const hardwareTypeOptions = Object.values(HardwareType).map(value => ({
  label: HARDWARE_TYPE_TEXT[value],
  value
}));
const selectedHardwareTypeOption = ref<{ label: string; value: HardwareType } | null>(null);
const selectedAttnImplementationOption = ref<{ label: string; value: AttentionImplementation } | null>(null);
const isLoading = ref(false);
const isSaving = ref(false);
const uiStore = useUiStore();
const settingsBusyLabel = computed(() => {
  if (isSaving.value) {
    return '正在保存配置与迁移目录，请稍候';
  }

  if (isLoading.value) {
    return '正在读取当前配置';
  }

  return '';
});

const canSaveConnection = computed(() => !isLoading.value && !isSaving.value);

const canSaveModel = computed(() => !isLoading.value && !isSaving.value);

const canSaveCache = computed(() => !isLoading.value && !isSaving.value);

const applySettings = (payload: SettingsForm) => {
  form.apiUrl = payload.apiUrl;
  form.apiToken = payload.apiToken;
  form.modelDir = payload.modelDir;
  form.dataDir = payload.dataDir;
  form.logCacheDir = payload.logCacheDir;
  form.hardwareType = payload.hardwareType;
  form.attnImplementation = payload.attnImplementation;
};

const loadSettings = async () => {
  isLoading.value = true;

  try {
    const payload = await invoke<SettingsResponse>('get_settings_config');
    applySettings(payload);
    uiStore.notifySuccess('已加载当前配置。', 2400);
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('读取配置失败，请检查 Rust 后端和配置文件', error));
  } finally {
    isLoading.value = false;
  }
};

const saveSettings = async (section: 'connection' | 'model' | 'cache') => {
  const isSectionValid = section === 'connection' ? canSaveConnection.value : section === 'model' ? canSaveModel.value : canSaveCache.value;

  if (!isSectionValid) {
    return;
  }

  isSaving.value = true;

  try {
    const payload = await invoke<SettingsResponse>('save_settings_config', {
      payload: {
        apiUrl: form.apiUrl,
        apiToken: form.apiToken,
        modelDir: form.modelDir,
        dataDir: form.dataDir,
        logCacheDir: form.logCacheDir,
        hardwareType: form.hardwareType,
        attnImplementation: form.attnImplementation
      }
    });
    applySettings(payload);
    uiStore.notifySuccess(section === 'connection' ? '连接配置已保存。' : section === 'model' ? '模型资源配置已保存。' : '缓存配置已保存。');
    if (payload.restartRequired && payload.migratedDirectories.length > 0) {
      const cleanupHint =
        payload.removableDirectories.length > 0
          ? `旧目录内容仍保留，可在确认新目录正常后手动删除：${payload.removableDirectories.join('；')}`
          : '模型旧目录内容已自动清理。';
      uiStore.notifyWarning(`已迁移${payload.migratedDirectories.join('、')}，请重启应用以切换到新目录。${cleanupHint}`, 7600);
    }
  } catch (error) {
    uiStore.notifyError(formatErrorMessage('保存配置失败，请检查填写内容或后端日志', error));
  } finally {
    isSaving.value = false;
  }
};

onMounted(async () => {
  await loadSettings();
});
</script>

<template>
  <div class="space-y-5">
    <PageHeader title="设置" description="管理服务连接、模型资源与数据日志路径。保存后会直接覆写 config.toml。" eyebrow="Settings" />

    <BaseLoadingBanner v-if="settingsBusyLabel" :label="settingsBusyLabel" />

    <PanelCard title="系统设置">
      <TabGroup>
        <TabList class="mb-4 flex flex-wrap gap-2">
          <Tab v-for="tab in settingTabs" :key="tab" v-slot="{ selected }" as="template">
            <button
              class="rounded-xl px-3 py-2 text-sm font-semibold transition"
              :class="selected ? 'bg-brand-500 text-white' : 'bg-brand-100/75 text-stone-700 hover:bg-brand-200/75'"
            >
              {{ tab }}
            </button>
          </Tab>
        </TabList>

        <TabPanels>
          <TabPanel class="space-y-3 text-sm text-slate-700">
            <label class="block">
              <span class="mb-1 block text-xs text-stone-500">Server URL</span>
              <input v-model="form.apiUrl" class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2" placeholder="请输入服务地址" />
            </label>
            <label class="block">
              <span class="mb-1 block text-xs text-stone-500">API Token</span>
              <input v-model="form.apiToken" class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2" placeholder="请输入访问令牌" />
            </label>
            <BaseButton :disabled="!canSaveConnection" @click="saveSettings('connection')">
              <BaseLoadingIndicator v-if="isSaving" size="sm" tone="muted" />
              <CheckBadgeIcon v-else class="h-4 w-4" aria-hidden="true" />
              <span>{{ isSaving ? '保存中...' : '保存连接配置' }}</span>
            </BaseButton>
          </TabPanel>

          <TabPanel class="space-y-3 text-sm text-slate-700">
            <p class="rounded-xl border border-brand-100 bg-brand-50/70 px-3 py-2 text-xs leading-5 text-stone-600">
              这里维护本地模型资源目录、全局硬件类型，以及统一的 Qwen 模型注意力实现。微调参数与 LoRA 配置已迁移到对应任务页面。
            </p>
            <label class="block">
              <span class="mb-1 block text-xs text-stone-500">模型目录</span>
              <input v-model="form.modelDir" class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2" placeholder="请输入模型目录" />
            </label>
            <BaseListbox
              v-model="form.hardwareType"
              v-model:selected-option="selectedHardwareTypeOption"
              label="全局硬件类型"
              :options="hardwareTypeOptions"
            />
            <BaseListbox
              v-model="form.attnImplementation"
              v-model:selected-option="selectedAttnImplementationOption"
              label="注意力实现"
              :options="attnImplementationOptions"
            />
            <BaseButton tone="ghost" :disabled="!canSaveModel" @click="saveSettings('model')">
              <BaseLoadingIndicator v-if="isSaving" size="sm" tone="muted" />
              <FolderIcon v-else class="h-4 w-4" aria-hidden="true" />
              <span>{{ isSaving ? '保存中...' : '保存资源配置' }}</span>
            </BaseButton>
          </TabPanel>

          <TabPanel class="space-y-3 text-sm text-slate-700">
            <section class="space-y-3 rounded-2xl border border-brand-100 bg-stone-50/80 p-4">
              <header class="space-y-1">
                <h3 class="text-sm font-semibold text-stone-700">缓存配置</h3>
                <p class="text-xs text-stone-500">数据目录同时作为训练缓存与本地业务数据根目录，日志目录单独配置。</p>
              </header>
              <label class="block">
                <span class="mb-1 block text-xs text-stone-500">数据目录</span>
                <input
                  v-model="form.dataDir"
                  class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
                  placeholder="请输入数据目录路径"
                />
              </label>
              <label class="block">
                <span class="mb-1 block text-xs text-stone-500">日志缓存路径</span>
                <input
                  v-model="form.logCacheDir"
                  class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
                  placeholder="请输入日志缓存路径"
                />
              </label>
            </section>
            <BaseButton tone="ghost" :disabled="!canSaveCache" @click="saveSettings('cache')">
              <BaseLoadingIndicator v-if="isSaving" size="sm" tone="muted" />
              <FolderIcon v-else class="h-4 w-4" aria-hidden="true" />
              <span>{{ isSaving ? '保存中...' : '保存缓存配置' }}</span>
            </BaseButton>
          </TabPanel>
        </TabPanels>
      </TabGroup>
    </PanelCard>
  </div>
</template>
