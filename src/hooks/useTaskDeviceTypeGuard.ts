import { computed, ref } from 'vue';

import { HARDWARE_TYPE_TEXT, HardwareType } from '@/enums/settings';
import { formatErrorMessage } from '@/hooks/useErrorMessage';
import { useModelStore } from '@/stores/models';
import { useUiStore } from '@/stores/ui';
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

const WARNING_MESSAGE =
  '当前选择硬件类型与模型环境当前支持的类型不一致，如果继续执行，会自动更新当前环境，从而大大延长本次任务执行的时间，是否继续？';

export const useTaskDeviceTypeGuard = () => {
  const modelStore = useModelStore();
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
      uiStore.notifyError(formatErrorMessage('查询模型当前环境硬件类型失败', error));
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
      `模型：${modelStore.getModelLabel(pendingMismatch.value.baseModel)} ${pendingMismatch.value.modelVersion}`,
      `当前环境类型：${HARDWARE_TYPE_TEXT[pendingMismatch.value.currentDevice]}`,
      `当前选择类型：${HARDWARE_TYPE_TEXT[pendingMismatch.value.selectedDevice]}`
    ];
  });

  return {
    dialogOpen: computed(() => pendingMismatch.value !== null),
    dialogTitle: '硬件环境确认',
    dialogMessage: WARNING_MESSAGE,
    dialogDetailLines,
    isCheckingDeviceType: computed(() => isCheckingDeviceType.value),
    isAwaitingDeviceConfirmation: computed(() => pendingMismatch.value !== null),
    isDeviceGuardPending: computed(() => isCheckingDeviceType.value || pendingMismatch.value !== null),
    ensureMatchedOrConfirmed,
    confirmDialog: () => resolvePending(true),
    closeDialog: () => resolvePending(false)
  };
};
