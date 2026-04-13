import { createRouter, createWebHashHistory } from 'vue-router';

import { HISTORY_TASK_ROUTE_PATH, HistoryTaskType } from '@/enums/task';
import HistoryView from '@/views/HistoryView.vue';
import ModelTrainingView from '@/views/ModelTrainingView.vue';
import NotFoundView from '@/views/NotFoundView.vue';
import SettingsView from '@/views/SettingsView.vue';
import SpeakersView from '@/views/SpeakersView.vue';
import TextToSpeechView from '@/views/TextToSpeechView.vue';
import VoiceCloneView from '@/views/VoiceCloneView.vue';

export const appRoutes = [
  {
    path: HISTORY_TASK_ROUTE_PATH[HistoryTaskType.ModelTraining],
    name: HistoryTaskType.ModelTraining,
    meta: { title: '模型训练' },
    component: ModelTrainingView
  },
  {
    path: HISTORY_TASK_ROUTE_PATH[HistoryTaskType.TextToSpeech],
    name: HistoryTaskType.TextToSpeech,
    meta: { title: '文本转语音' },
    component: TextToSpeechView
  },
  {
    path: HISTORY_TASK_ROUTE_PATH[HistoryTaskType.VoiceClone],
    name: HistoryTaskType.VoiceClone,
    meta: { title: '声音克隆' },
    component: VoiceCloneView
  },
  {
    path: '/speakers',
    name: 'speakers',
    meta: { title: '说话人管理' },
    component: SpeakersView
  },
  {
    path: '/history',
    name: 'history',
    meta: { title: '历史任务' },
    component: HistoryView
  },
  {
    path: '/settings',
    name: 'settings',
    meta: { title: '设置' },
    component: SettingsView
  },
  {
    path: '/',
    redirect: HISTORY_TASK_ROUTE_PATH[HistoryTaskType.ModelTraining]
  },
  {
    path: '/:pathMatch(.*)*',
    name: 'not-found',
    meta: { title: '页面不存在' },
    component: NotFoundView
  }
];

const router = createRouter({
  history: createWebHashHistory(),
  routes: appRoutes
});

export default router;
