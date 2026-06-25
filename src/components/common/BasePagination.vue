<script setup lang="ts">
import { computed, ref, watch } from 'vue';

import { ChevronDoubleLeftIcon, ChevronDoubleRightIcon, ChevronLeftIcon, ChevronRightIcon } from '@heroicons/vue/24/outline';

import BaseListbox from '@/components/common/BaseListbox.vue';

interface Props {
  currentPage: number;
  pageSize: number;
  totalItems: number;
  pageSizeOptions?: number[];
  siblingCount?: number;
  disabled?: boolean;
  loading?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  pageSizeOptions: () => [10, 20, 50],
  siblingCount: 1,
  disabled: false,
  loading: false
});

const emit = defineEmits<{
  'update:currentPage': [page: number];
  'update:pageSize': [size: number];
}>();

const totalPages = computed(() => Math.max(1, Math.ceil(props.totalItems / props.pageSize)));

const pageSizeOptions = computed(() => props.pageSizeOptions.map(size => ({ label: `${size} 条/页`, value: size })));

type PageToken = number | 'ellipsis';

const pageTokens = computed<PageToken[]>(() => {
  const total = totalPages.value;
  const current = props.currentPage;
  const sibling = props.siblingCount;
  const tokens: PageToken[] = [1];

  const left = Math.max(2, current - sibling);
  const right = Math.min(total - 1, current + sibling);

  if (left > 2) {
    tokens.push('ellipsis');
  }
  for (let page = left; page <= right; page += 1) {
    tokens.push(page);
  }
  if (right < total - 1) {
    tokens.push('ellipsis');
  }
  if (total > 1) {
    tokens.push(total);
  }
  return tokens;
});

const rangeStart = computed(() => (props.totalItems === 0 ? 0 : (props.currentPage - 1) * props.pageSize + 1));
const rangeEnd = computed(() => Math.min(props.currentPage * props.pageSize, props.totalItems));

const clampPage = (page: number) => Math.min(Math.max(1, page), totalPages.value);

const goTo = (page: number) => {
  const target = clampPage(page);
  if (target !== props.currentPage) {
    emit('update:currentPage', target);
  }
};

const onPageSizeChange = (size: number) => {
  if (size !== props.pageSize) {
    emit('update:pageSize', size);
  }
};

const jumpInput = ref('');

const commitJump = () => {
  const parsed = Number.parseInt(jumpInput.value, 10);
  if (Number.isFinite(parsed)) {
    goTo(parsed);
  }
  jumpInput.value = '';
};

watch(totalPages, total => {
  if (props.currentPage > total) {
    emit('update:currentPage', total);
  }
});

const navButtonClass =
  'inline-flex h-8 w-8 items-center justify-center rounded-xl border border-brand-200 bg-white/90 text-brand-700 transition hover:-translate-y-0.5 hover:bg-brand-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-200 disabled:cursor-not-allowed disabled:opacity-55 disabled:hover:translate-y-0 disabled:hover:bg-white/90';
const pageButtonClass =
  'inline-flex h-8 min-w-8 items-center justify-center rounded-xl border px-2 text-xs font-semibold transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-200 disabled:cursor-not-allowed disabled:opacity-55';
</script>

<template>
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div class="flex items-center gap-3">
      <span class="text-xs text-stone-500">
        共 <span class="font-semibold text-stone-700">{{ totalItems }}</span> 条
        <template v-if="totalItems > 0">· 第 {{ rangeStart }}-{{ rangeEnd }} 条</template>
      </span>
      <div class="w-32">
        <BaseListbox
          :model-value="pageSize"
          :options="pageSizeOptions"
          :disabled="disabled || loading"
          @update:model-value="onPageSizeChange($event as number)"
        />
      </div>
    </div>

    <div class="flex items-center gap-1.5">
      <button type="button" :disabled="disabled || loading || currentPage <= 1" :class="navButtonClass" aria-label="第一页" @click="goTo(1)">
        <ChevronDoubleLeftIcon class="h-4 w-4" aria-hidden="true" />
      </button>
      <button
        type="button"
        :disabled="disabled || loading || currentPage <= 1"
        :class="navButtonClass"
        aria-label="上一页"
        @click="goTo(currentPage - 1)"
      >
        <ChevronLeftIcon class="h-4 w-4" aria-hidden="true" />
      </button>

      <template v-for="(token, index) in pageTokens" :key="`${token}-${index}`">
        <span v-if="token === 'ellipsis'" class="inline-flex h-8 items-center px-1 text-xs text-stone-400">…</span>
        <button
          v-else
          type="button"
          :disabled="disabled || loading"
          :class="[
            pageButtonClass,
            token === currentPage
              ? 'border-brand-500 bg-brand-500 text-white shadow-[0_8px_18px_rgba(216,115,39,0.22)]'
              : 'border-brand-200 bg-white/90 text-brand-700 hover:-translate-y-0.5 hover:bg-brand-50'
          ]"
          @click="goTo(token)"
        >
          {{ token }}
        </button>
      </template>

      <button
        type="button"
        :disabled="disabled || loading || currentPage >= totalPages"
        :class="navButtonClass"
        aria-label="下一页"
        @click="goTo(currentPage + 1)"
      >
        <ChevronRightIcon class="h-4 w-4" aria-hidden="true" />
      </button>
      <button
        type="button"
        :disabled="disabled || loading || currentPage >= totalPages"
        :class="navButtonClass"
        aria-label="最后一页"
        @click="goTo(totalPages)"
      >
        <ChevronDoubleRightIcon class="h-4 w-4" aria-hidden="true" />
      </button>

      <span class="ml-1 flex items-center gap-1 text-xs text-stone-500">
        前往
        <input
          v-model="jumpInput"
          type="number"
          min="1"
          :max="totalPages"
          :disabled="disabled || loading"
          class="w-12 h-8 rounded-xl border border-brand-200 bg-white/90 px-2 py-1 text-center text-xs text-slate-700 shadow-[0_6px_18px_rgba(200,124,57,0.08)] transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-200 disabled:cursor-not-allowed disabled:opacity-55"
          @keyup.enter="commitJump"
          @blur="commitJump"
        />
        页
      </span>
    </div>
  </div>
</template>
