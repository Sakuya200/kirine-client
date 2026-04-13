import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

type NoticeTone = 'success' | 'error' | 'info' | 'warning';

interface NoticeInput {
  message: string;
  tone?: NoticeTone;
  duration?: number;
}

interface NoticeItem {
  id: number;
  message: string;
  tone: NoticeTone;
}

export const useUiStore = defineStore('ui', () => {
  const sidebarCollapsed = ref(false);
  const notices = ref<NoticeItem[]>([]);
  const timers = new Map<number, number>();
  let noticeSeed = 0;

  const sidebarWidth = computed(() => (sidebarCollapsed.value ? 72 : 276));
  const sidebarWidthClass = computed(() => (sidebarCollapsed.value ? 'w-[72px]' : 'w-[276px]'));

  const toggleSidebar = () => {
    sidebarCollapsed.value = !sidebarCollapsed.value;
  };

  const removeNotice = (id: number) => {
    notices.value = notices.value.filter(item => item.id !== id);

    const timer = timers.get(id);

    if (timer) {
      window.clearTimeout(timer);
      timers.delete(id);
    }
  };

  const notify = ({ message, tone = 'info', duration = 3600 }: NoticeInput) => {
    const trimmedMessage = message.trim();

    if (!trimmedMessage) {
      return;
    }

    const id = ++noticeSeed;
    const nextItems = [{ id, message: trimmedMessage, tone }, ...notices.value];
    const overflowItems = nextItems.slice(4);

    notices.value = nextItems.slice(0, 4);
    overflowItems.forEach(item => removeNotice(item.id));

    const timer = window.setTimeout(() => removeNotice(id), duration);
    timers.set(id, timer);
  };

  const notifySuccess = (message: string, duration?: number) => notify({ message, tone: 'success', duration });
  const notifyError = (message: string, duration = 4800) => notify({ message, tone: 'error', duration });
  const notifyInfo = (message: string, duration?: number) => notify({ message, tone: 'info', duration });
  const notifyWarning = (message: string, duration = 4200) => notify({ message, tone: 'warning', duration });

  return {
    sidebarCollapsed,
    notices,
    sidebarWidth,
    sidebarWidthClass,
    notify,
    notifyError,
    notifyInfo,
    notifySuccess,
    notifyWarning,
    removeNotice,
    toggleSidebar
  };
});
