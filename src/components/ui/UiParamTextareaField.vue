<script setup lang="ts">
interface Props {
  modelValue: string;
  label: string;
  description?: string;
  rows?: number;
  placeholder?: string;
}

withDefaults(defineProps<Props>(), {
  description: '',
  rows: 3,
  placeholder: undefined
});

const emit = defineEmits<{
  'update:modelValue': [value: string];
}>();

const onInput = (event: Event) => {
  emit('update:modelValue', (event.target as HTMLTextAreaElement).value);
};
</script>

<template>
  <label class="block md:col-span-2">
    <span class="mb-1 block text-xs text-stone-500">{{ label }}</span>
    <textarea
      :value="modelValue"
      :rows="rows"
      :placeholder="placeholder"
      class="min-h-[96px] w-full rounded-xl border border-brand-200 bg-white/90 px-3 py-2"
      @input="onInput"
    />
    <p v-if="description" class="mt-1 text-xs leading-5 text-stone-500">{{ description }}</p>
  </label>
</template>
