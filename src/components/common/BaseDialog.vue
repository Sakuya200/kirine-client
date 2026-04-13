<script setup lang="ts">
import { Dialog, DialogPanel, DialogTitle, TransitionChild, TransitionRoot } from '@headlessui/vue';

interface Props {
  open: boolean;
  title: string;
  panelClass?: string;
  contentClass?: string;
  zClass?: string;
}

withDefaults(defineProps<Props>(), {
  panelClass: 'max-w-lg',
  contentClass: '',
  zClass: 'z-[120]'
});

const emit = defineEmits<{
  close: [];
}>();
</script>

<template>
  <TransitionRoot :show="open" as="template">
    <Dialog class="relative" :class="zClass" @close="emit('close')">
      <TransitionChild
        as="template"
        enter="transition-opacity duration-200"
        enter-from="opacity-0"
        enter-to="opacity-100"
        leave="transition-opacity duration-150"
        leave-from="opacity-100"
        leave-to="opacity-0"
      >
        <div class="fixed inset-0 bg-[#7a4a24]/18 backdrop-blur-[2px]" aria-hidden="true" />
      </TransitionChild>

      <div class="fixed inset-0 flex items-end justify-center p-3 sm:items-center sm:p-4">
        <TransitionChild
          as="template"
          enter="transition duration-200 ease-out"
          enter-from="translate-y-2 scale-[0.98] opacity-0"
          enter-to="translate-y-0 scale-100 opacity-100"
          leave="transition duration-150 ease-in"
          leave-from="translate-y-0 scale-100 opacity-100"
          leave-to="translate-y-2 scale-[0.98] opacity-0"
        >
          <DialogPanel
            class="w-full max-h-[calc(100vh-1.5rem)] overflow-y-auto rounded-2xl border border-brand-100 bg-[#fffdfa] p-5 shadow-panel sm:max-h-[calc(100vh-2rem)] sm:p-6"
            :class="panelClass"
          >
            <DialogTitle class="text-lg font-semibold text-slate-900">{{ title }}</DialogTitle>
            <div class="mt-4" :class="contentClass">
              <slot />
            </div>
            <div v-if="$slots.footer" class="mt-5 flex justify-end gap-2">
              <slot name="footer" />
            </div>
          </DialogPanel>
        </TransitionChild>
      </div>
    </Dialog>
  </TransitionRoot>
</template>
