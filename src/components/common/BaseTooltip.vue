<script setup lang="ts">
import { TransitionChild, TransitionRoot } from '@headlessui/vue';
import { QuestionMarkCircleIcon } from '@heroicons/vue/24/outline';
import { computed, ref, useSlots } from 'vue';

interface Props {
  text?: string;
  title?: string;
  placement?: 'top' | 'right' | 'bottom' | 'left';
  iconClass?: string;
  panelClass?: string;
  disabled?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  text: '',
  title: '',
  placement: 'top',
  iconClass: 'h-4.5 w-4.5',
  panelClass: '',
  disabled: false
});

const open = ref(false);

const hasContent = computed(() => Boolean(props.title.trim() || props.text.trim() || !!useSlots().default));

const placementClass = computed(() => {
  switch (props.placement) {
    case 'right':
      return 'left-full top-1/2 ml-3 -translate-y-1/2';
    case 'bottom':
      return 'left-1/2 top-full mt-3 -translate-x-1/2';
    case 'left':
      return 'right-full top-1/2 mr-3 -translate-y-1/2';
    case 'top':
    default:
      return 'bottom-full left-1/2 mb-3 -translate-x-1/2';
  }
});

const arrowClass = computed(() => {
  switch (props.placement) {
    case 'right':
      return '-left-1 top-1/2 -translate-y-1/2 rotate-45';
    case 'bottom':
      return 'left-1/2 -top-1 -translate-x-1/2 rotate-45';
    case 'left':
      return '-right-1 top-1/2 -translate-y-1/2 rotate-45';
    case 'top':
    default:
      return 'bottom-[-0.25rem] left-1/2 -translate-x-1/2 rotate-45';
  }
});

const show = () => {
  if (props.disabled || !hasContent.value) {
    return;
  }

  open.value = true;
};

const hide = () => {
  open.value = false;
};
</script>

<template>
  <span class="relative inline-flex align-middle" @mouseenter="show" @mouseleave="hide">
    <button
      type="button"
      class="inline-flex h-5 w-5 items-center justify-center rounded-full text-brand-500 transition-colors duration-200 hover:text-brand-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-200 focus-visible:ring-offset-2"
      :disabled="disabled || !hasContent"
      :aria-label="title || text || '显示提示信息'"
      @focus="show"
      @blur="hide"
    >
      <slot name="trigger">
        <QuestionMarkCircleIcon :class="iconClass" aria-hidden="true" />
      </slot>
    </button>

    <TransitionRoot :show="open" as="template">
      <TransitionChild
        as="template"
        enter="transition duration-150 ease-out"
        enter-from="translate-y-1 scale-95 opacity-0"
        enter-to="translate-y-0 scale-100 opacity-100"
        leave="transition duration-100 ease-in"
        leave-from="translate-y-0 scale-100 opacity-100"
        leave-to="translate-y-1 scale-95 opacity-0"
      >
        <div class="pointer-events-none absolute z-50 w-64 max-w-[min(18rem,calc(100vw-2rem))]" :class="[placementClass, panelClass]" role="tooltip">
          <div
            class="relative rounded-2xl border border-brand-200/80 bg-[#fff9f3]/98 p-3 text-left shadow-[0_18px_36px_rgba(117,73,35,0.16)] backdrop-blur-sm"
          >
            <span class="absolute h-2 w-2 border border-brand-200/80 bg-[#fff9f3]" :class="arrowClass" aria-hidden="true" />
            <p v-if="title" class="text-xs font-semibold tracking-[0.08em] text-slate-900">{{ title }}</p>
            <p v-if="text" class="text-xs leading-5 text-slate-600" :class="title ? 'mt-1' : ''">{{ text }}</p>
            <div v-if="$slots.default" class="text-xs leading-5 text-slate-600" :class="title || text ? 'mt-1.5' : ''">
              <slot />
            </div>
          </div>
        </div>
      </TransitionChild>
    </TransitionRoot>
  </span>
</template>
