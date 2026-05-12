<script setup lang="ts">
interface Props {
  title?: string;
  description?: string;
}

withDefaults(defineProps<Props>(), {
  title: '当前模型没有特有参数',
  description: '这个任务下没有额外参数需要配置，可以直接继续后续操作。'
});
</script>

<template>
  <div class="rounded-2xl border border-dashed border-brand-200 bg-brand-50/30 px-4 py-5 text-sm text-stone-600">
    <p class="font-medium text-slate-800">{{ title }}</p>
    <p class="mt-1 leading-6">{{ description }}</p>
  </div>
</template>
