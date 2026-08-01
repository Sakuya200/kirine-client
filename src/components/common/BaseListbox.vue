<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue';

import { Listbox, ListboxButton, ListboxLabel, ListboxOption, ListboxOptions } from '@headlessui/vue';
import { ChevronUpDownIcon } from '@heroicons/vue/20/solid';

type ListboxValue = string | number | boolean | null | undefined;
type ListOption = object;

interface Props {
  modelValue: ListboxValue;
  options: ListOption[];
  label?: string;
  optionLabelKey?: string;
  optionValueKey?: string;
  placeholder?: string;
  disabled?: boolean;
  /** 将下拉面板 Teleport 到 body 并以 fixed 定位跟随触发按钮，避免被表格 overflow 等祖先裁切。默认 false（保持原内联行为）。 */
  teleport?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  optionLabelKey: 'label',
  optionValueKey: 'value',
  placeholder: '请选择',
  teleport: false,
});

const emit = defineEmits<{
  'update:modelValue': [value: ListboxValue];
  'update:selectedOption': [option: ListOption | null];
}>();

const getOptionRecord = (option: ListOption) => option as Record<string, unknown>;

const getOptionValue = (option: ListOption): ListboxValue => getOptionRecord(option)[props.optionValueKey] as ListboxValue;

const getOptionLabel = (option: ListOption): string => String(getOptionRecord(option)[props.optionLabelKey] ?? '');

const selectedOption = computed(() => props.options.find(option => getOptionValue(option) === props.modelValue) ?? null);

watch(
  selectedOption,
  option => {
    emit('update:selectedOption', option);
  },
  { immediate: true }
);

const updateValue = (value: ListboxValue) => {
  emit('update:modelValue', value);
  emit('update:selectedOption', props.options.find(option => getOptionValue(option) === value) ?? null);
};

const buttonClass =
  'relative w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2 pr-10 text-left text-sm text-slate-700 shadow-[0_6px_18px_rgba(200,124,57,0.08)] transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-200';

const disabledButtonClass = 'cursor-not-allowed border-stone-200 bg-stone-50/90 text-slate-500 shadow-none';

// teleport 定位：开启 teleport 时，下拉面板脱离文档流以 fixed 定位跟随触发按钮。
const wrapperRef = ref<HTMLElement | null>(null);
const isOpen = ref(false);
const panelStyle = ref<Record<string, string>>({});

// HeadlessUI Listbox 不暴露 open 事件；借 slot 的 open 值在每次渲染同步到 ref。
const syncOpen = (open: boolean): string => {
  if (open !== isOpen.value) {
    isOpen.value = open;
  }
  return '';
};

const computePanelStyle = () => {
  const button = wrapperRef.value?.querySelector('button');
  if (!button) return;
  const rect = button.getBoundingClientRect();
  panelStyle.value = {
    position: 'fixed',
    top: `${rect.bottom + 4}px`,
    left: `${rect.left}px`,
    width: `${rect.width}px`,
  };
};

const onScrollOrResize = () => {
  if (isOpen.value) {
    computePanelStyle();
  }
};

watch(isOpen, open => {
  if (!props.teleport) return;
  if (open) {
    nextTick(computePanelStyle);
    window.addEventListener('scroll', onScrollOrResize, true);
    window.addEventListener('resize', onScrollOrResize);
  } else {
    window.removeEventListener('scroll', onScrollOrResize, true);
    window.removeEventListener('resize', onScrollOrResize);
  }
});

onBeforeUnmount(() => {
  window.removeEventListener('scroll', onScrollOrResize, true);
  window.removeEventListener('resize', onScrollOrResize);
});
</script>

<template>
  <Listbox v-slot="{ open }" :model-value="modelValue" :disabled="disabled" @update:model-value="updateValue">
    <div ref="wrapperRef" class="relative" :class="open ? 'z-[120]' : 'z-auto'">
      <span class="sr-only" aria-hidden="true">{{ syncOpen(open) }}</span>
      <ListboxLabel v-if="label" class="mb-1 block text-xs text-stone-500">{{ label }}</ListboxLabel>
      <ListboxButton :class="[buttonClass, disabled ? disabledButtonClass : '']">
        <span class="block truncate">{{ selectedOption ? getOptionLabel(selectedOption) : placeholder }}</span>
        <span class="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2">
          <ChevronUpDownIcon class="h-5 w-5" :class="disabled ? 'text-stone-400' : 'text-brand-500'" aria-hidden="true" />
        </span>
      </ListboxButton>
      <Teleport to="body" :disabled="!teleport">
        <ListboxOptions
          v-if="!disabled"
          class="max-h-60 overflow-auto rounded-xl border border-brand-200 bg-[#fffdfa] p-1 shadow-soft focus:outline-none"
          :class="teleport ? 'fixed z-[130]' : 'absolute left-0 right-0 z-[130] mt-1'"
          :style="teleport ? panelStyle : undefined"
        >
          <ListboxOption
            v-for="option in options"
            :key="String(getOptionValue(option))"
            v-slot="{ active, selected }"
            :value="getOptionValue(option)"
            as="template"
          >
            <li
              class="cursor-pointer rounded-lg px-3 py-2 text-sm transition"
              :class="active || selected ? 'bg-brand-50 text-brand-800' : 'text-slate-700'"
            >
              {{ getOptionLabel(option) }}
            </li>
          </ListboxOption>
        </ListboxOptions>
      </Teleport>
    </div>
  </Listbox>
</template>
