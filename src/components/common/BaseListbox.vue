<script setup lang="ts">
import { computed, watch } from 'vue';

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
}

const props = withDefaults(defineProps<Props>(), {
  optionLabelKey: 'label',
  optionValueKey: 'value',
  placeholder: '请选择'
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
</script>

<template>
  <Listbox v-slot="{ open }" :model-value="modelValue" :disabled="disabled" @update:model-value="updateValue">
    <div class="relative" :class="open ? 'z-[120]' : 'z-auto'">
      <ListboxLabel v-if="label" class="mb-1 block text-xs text-stone-500">{{ label }}</ListboxLabel>
      <ListboxButton :class="[buttonClass, disabled ? disabledButtonClass : '']">
        <span class="block truncate">{{ selectedOption ? getOptionLabel(selectedOption) : placeholder }}</span>
        <span class="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2">
          <ChevronUpDownIcon class="h-5 w-5" :class="disabled ? 'text-stone-400' : 'text-brand-500'" aria-hidden="true" />
        </span>
      </ListboxButton>
      <ListboxOptions
        v-if="!disabled"
        class="absolute left-0 right-0 z-[130] mt-1 max-h-60 overflow-auto rounded-xl border border-brand-200 bg-[#fffdfa] p-1 shadow-soft focus:outline-none"
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
    </div>
  </Listbox>
</template>
