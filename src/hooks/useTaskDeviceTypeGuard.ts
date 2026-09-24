import { computed, ref } from 'vue';

import { HARDWARE_TYPE_TEXT, HardwareType } from '@/enums/settings';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { useModels } from '@/hooks/useModels';
import { useModelStore } from '@/stores/models';
import { useUiStore } from '@/stores/ui';
import { i18n } from '@/locales';
import type { BaseModel } from '@/types/domain';

interface EnsureTaskDeviceInput {
  baseModel: BaseModel;
  modelVersion: string;
  selectedDevice: HardwareType;
}

interface PendingDeviceMismatch {
  baseModel: BaseModel;
  modelVersion: string;
  currentDevice: HardwareType;
  selectedDevice: HardwareType;
}

const warningMessage = () => i18n.global.t('common.deviceGuard.message');

export const useTaskDeviceTypeGuard = () => {
  const modelStore = useModelStore();
  const { getModelLabel } = useModels();
  const uiStore = useUiStore();
  const isCheckingDeviceType = ref(false);
  const pendingMismatch = ref<PendingDeviceMismatch | null>(null);
  let pendingResolver: ((confirmed: boolean) => void) | null = null;

  const resolvePending = (confirmed: boolean) => {
    pendingMismatch.value = null;
    const resolver = pendingResolver;
    pendingResolver = null;
    resolver?.(confirmed);
  };

  const ensureMatchedOrConfirmed = async ({ baseModel, modelVersion, selectedDevice }: EnsureTaskDeviceInput) => {
    isCheckingDeviceType.value = true;

    try {
      const currentDevice = await modelStore.getDeviceType(baseModel, modelVersion);
      if (currentDevice === selectedDevice) {
        return true;
      }

      if (pendingResolver) {
        pendingResolver(false);
        pendingResolver = null;
      }

      pendingMismatch.value = {
        baseModel,
        modelVersion,
        currentDevice,
        selectedDevice
      };

      return await new Promise<boolean>(resolve => {
        pendingResolver = resolve;
      });
    } catch (error) {
      uiStore.notifyError(formatErrorMessage(i18n.global.t('common.deviceGuard.checkFailed'), error));
      return false;
    } finally {
      isCheckingDeviceType.value = false;
    }
  };

  const dialogDetailLines = computed(() => {
    if (!pendingMismatch.value) {
      return [];
    }

    return [
      i18n.global.t('common.deviceGuard.model', { model: getModelLabel(pendingMismatch.value.baseModel), version: pendingMismatch.value.modelVersion }),
      i18n.global.t('common.deviceGuard.currentEnvType', { device: HARDWARE_TYPE_TEXT[pendingMismatch.value.currentDevice] }),
      i18n.global.t('common.deviceGuard.selectedType', { device: HARDWARE_TYPE_TEXT[pendingMismatch.value.selectedDevice] })
    ];
  });

  return {
    dialogOpen: computed(() => pendingMismatch.value !== null),
    dialogTitle: i18n.global.t('common.deviceGuard.title'),
    dialogMessage: warningMessage(),
    dialogDetailLines,
    isCheckingDeviceType: computed(() => isCheckingDeviceType.value),
    isAwaitingDeviceConfirmation: computed(() => pendingMismatch.value !== null),
    isDeviceGuardPending: computed(() => isCheckingDeviceType.value || pendingMismatch.value !== null),
    ensureMatchedOrConfirmed,
    confirmDialog: () => resolvePending(true),
    closeDialog: () => resolvePending(false)
  };
};
