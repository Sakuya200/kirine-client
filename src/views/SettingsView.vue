<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { CheckBadgeIcon, FolderIcon } from '@heroicons/vue/24/outline';
import { Tab, TabGroup, TabList, TabPanel, TabPanels } from '@headlessui/vue';
import { computed, onMounted, reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';

import BaseButton from '@/components/common/BaseButton.vue';
import BaseLoadingBanner from '@/components/common/BaseLoadingBanner.vue';
import BaseListbox from '@/components/common/BaseListbox.vue';
import PageHeader from '@/components/common/PageHeader.vue';
import PanelCard from '@/components/common/PanelCard.vue';
import { UiLanguage, UI_LANGUAGE_OPTIONS } from '@/enums/uiLanguage';
import { ATTENTION_IMPLEMENTATION_TEXT, AttentionImplementation } from '@/enums/settings';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { setUiLanguage } from '@/hooks/useUiLanguage';
import { i18n } from '@/locales';
import { useUiStore } from '@/stores/ui';

const { t } = useI18n();

const settingTabs = computed(() => [t('settings.tabs.connection'), t('settings.tabs.model'), t('settings.tabs.cache')]);

interface SettingsForm {
  apiUrl: string;
  apiToken: string;
  modelDir: string;
  dataDir: string;
  logCacheDir: string;
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
  attnImplementation: AttentionImplementation.Sdpa
};

const form = reactive<SettingsForm>({ ...DEFAULT_SETTINGS_FORM });
// 注意力实现为技术术语（SDPA 等），语言无关，不走 i18n
const attnImplementationOptions = Object.values(AttentionImplementation).map(value => ({
  label: ATTENTION_IMPLEMENTATION_TEXT[value],
  value
}));
const selectedAttnImplementationOption = ref<{ label: string; value: AttentionImplementation } | null>(null);
const isLoading = ref(false);
const isSaving = ref(false);
const uiStore = useUiStore();
const settingsBusyLabel = computed(() => {
  if (isSaving.value) {
    return t('settings.loading.saving');
  }

  if (isLoading.value) {
    return t('settings.loading.reading');
  }

  return '';
});

const canSaveConnection = computed(() => !isLoading.value && !isSaving.value);

const currentLanguage = computed(() => i18n.global.locale.value as UiLanguage);

const listSeparator = computed(() => (currentLanguage.value === UiLanguage.English ? '; ' : '；'));

const onLanguageChange = async (value: UiLanguage | string | number | boolean | null | undefined) => {
  if (typeof value === 'string') {
    await setUiLanguage(value as UiLanguage);
  }
};

const canSaveModel = computed(() => !isLoading.value && !isSaving.value);

const canSaveCache = computed(() => !isLoading.value && !isSaving.value);

const applySettings = (payload: SettingsForm) => {
  form.apiUrl = payload.apiUrl;
  form.apiToken = payload.apiToken;
  form.modelDir = payload.modelDir;
  form.dataDir = payload.dataDir;
  form.logCacheDir = payload.logCacheDir;
  form.attnImplementation = payload.attnImplementation;
};

const loadSettings = async () => {
  isLoading.value = true;

  try {
    const payload = await invoke<SettingsResponse>('get_settings_config');
    applySettings(payload);
    uiStore.notifySuccess(t('settings.notice.loaded'), 2400);
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('settings.notice.loadFailed'), error));
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
        attnImplementation: form.attnImplementation
      }
    });
    applySettings(payload);
    uiStore.notifySuccess(
      section === 'connection' ? t('settings.notice.savedConnection') : section === 'model' ? t('settings.notice.savedModel') : t('settings.notice.savedCache')
    );
    if (payload.restartRequired && payload.migratedDirectories.length > 0) {
      const cleanupHint =
        payload.removableDirectories.length > 0
          ? t('settings.notice.cleanupHint', { dirs: payload.removableDirectories.join(listSeparator.value) })
          : t('settings.notice.cleanedHint');
      uiStore.notifyWarning(t('settings.notice.migrated', { dirs: payload.migratedDirectories.join(listSeparator.value) }) + cleanupHint, 7600);
    }
  } catch (error) {
    uiStore.notifyError(formatErrorMessage(t('settings.notice.saveFailed'), error));
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
    <PageHeader :title="t('settings.title')" :description="t('settings.description')" eyebrow="Settings" />

    <BaseLoadingBanner v-if="settingsBusyLabel" :label="settingsBusyLabel" />

    <PanelCard :title="t('settings.panelTitle')">
      <div class="mb-4 flex items-center gap-3">
        <span class="text-sm text-stone-600">{{ t('settings.uiLanguage.label') }}</span>
        <div class="w-48">
          <BaseListbox
            :model-value="currentLanguage"
            :options="UI_LANGUAGE_OPTIONS"
            @update:model-value="onLanguageChange"
          />
        </div>
      </div>

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
              <span class="mb-1 block text-xs text-stone-500">{{ t('settings.connection.serverUrl') }}</span>
              <input v-model="form.apiUrl" class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2" :placeholder="t('settings.connection.serverUrlPlaceholder')" />
            </label>
            <label class="block">
              <span class="mb-1 block text-xs text-stone-500">{{ t('settings.connection.apiToken') }}</span>
              <input v-model="form.apiToken" class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2" :placeholder="t('settings.connection.apiTokenPlaceholder')" />
            </label>
            <BaseButton :loading="isSaving" :disabled="!canSaveConnection" @click="saveSettings('connection')">
              <CheckBadgeIcon v-if="!isSaving" class="h-4 w-4" aria-hidden="true" />
              <span>{{ isSaving ? t('common.saving') : t('settings.connection.save') }}</span>
            </BaseButton>
          </TabPanel>

          <TabPanel class="space-y-3 text-sm text-slate-700">
            <p class="rounded-xl border border-brand-100 bg-brand-50/70 px-3 py-2 text-xs leading-5 text-stone-600">
              {{ t('settings.model.hint') }}
            </p>
            <label class="block">
              <span class="mb-1 block text-xs text-stone-500">{{ t('settings.model.modelDir') }}</span>
              <input v-model="form.modelDir" class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2" :placeholder="t('settings.model.modelDirPlaceholder')" />
            </label>
            <BaseListbox
              v-model="form.attnImplementation"
              v-model:selected-option="selectedAttnImplementationOption"
              :label="t('settings.model.attnLabel')"
              :options="attnImplementationOptions"
            />
            <BaseButton tone="ghost" :loading="isSaving" :disabled="!canSaveModel" @click="saveSettings('model')">
              <FolderIcon v-if="!isSaving" class="h-4 w-4" aria-hidden="true" />
              <span>{{ isSaving ? t('common.saving') : t('settings.model.save') }}</span>
            </BaseButton>
          </TabPanel>

          <TabPanel class="space-y-3 text-sm text-slate-700">
            <section class="space-y-3 rounded-2xl border border-brand-100 bg-stone-50/80 p-4">
              <header class="space-y-1">
                <h3 class="text-sm font-semibold text-stone-700">{{ t('settings.cache.header') }}</h3>
                <p class="text-xs text-stone-500">{{ t('settings.cache.hint') }}</p>
              </header>
              <label class="block">
                <span class="mb-1 block text-xs text-stone-500">{{ t('settings.cache.dataDir') }}</span>
                <input
                  v-model="form.dataDir"
                  class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
                  :placeholder="t('settings.cache.dataDirPlaceholder')"
                />
              </label>
              <label class="block">
                <span class="mb-1 block text-xs text-stone-500">{{ t('settings.cache.logDir') }}</span>
                <input
                  v-model="form.logCacheDir"
                  class="w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
                  :placeholder="t('settings.cache.logDirPlaceholder')"
                />
              </label>
            </section>
            <BaseButton tone="ghost" :loading="isSaving" :disabled="!canSaveCache" @click="saveSettings('cache')">
              <FolderIcon v-if="!isSaving" class="h-4 w-4" aria-hidden="true" />
              <span>{{ isSaving ? t('common.saving') : t('settings.cache.save') }}</span>
            </BaseButton>
          </TabPanel>
        </TabPanels>
      </TabGroup>
    </PanelCard>
  </div>
</template>
