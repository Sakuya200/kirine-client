import { createRouter, createWebHashHistory } from 'vue-router';

import { HISTORY_TASK_ROUTE_PATH, HistoryTaskType } from '@/enums/task';
import HistoryView from '@/views/HistoryView.vue';
import ModelManageView from '@/views/ModelManageView.vue';
import ModelTrainingView from '@/views/ModelTrainingView.vue';
import NotFoundView from '@/views/NotFoundView.vue';
import SettingsView from '@/views/SettingsView.vue';
import SpeakersView from '@/views/SpeakersView.vue';
import StreamingSpeechView from '@/views/StreamingSpeechView.vue';
import TextToSpeechView from '@/views/TextToSpeechView.vue';
import VoiceDesignView from '@/views/VoiceDesignView.vue';
import VoiceCloneView from '@/views/VoiceCloneView.vue';

export const appRoutes = [
  {
    path: HISTORY_TASK_ROUTE_PATH[HistoryTaskType.ModelTraining],
    name: HistoryTaskType.ModelTraining,
    meta: { title: 'common.historyTaskType.modelTraining' },
    component: ModelTrainingView
  },
  {
    path: HISTORY_TASK_ROUTE_PATH[HistoryTaskType.TextToSpeech],
    name: HistoryTaskType.TextToSpeech,
    meta: { title: 'common.historyTaskType.textToSpeech' },
    component: TextToSpeechView
  },
  {
    path: HISTORY_TASK_ROUTE_PATH[HistoryTaskType.VoiceClone],
    name: HistoryTaskType.VoiceClone,
    meta: { title: 'common.historyTaskType.voiceClone' },
    component: VoiceCloneView
  },
  {
    path: HISTORY_TASK_ROUTE_PATH[HistoryTaskType.VoiceDesign],
    name: HistoryTaskType.VoiceDesign,
    meta: { title: 'common.historyTaskType.voiceDesign' },
    component: VoiceDesignView
  },
  {
    path: '/streaming-speech',
    name: 'streaming-speech',
    meta: { title: 'common.historyTaskType.streamingSpeech' },
    component: StreamingSpeechView
  },
  {
    path: '/models',
    name: 'models',
    meta: { title: 'modelManage.title' },
    component: ModelManageView
  },
  {
    path: '/speakers',
    name: 'speakers',
    meta: { title: 'speakers.title' },
    component: SpeakersView
  },
  {
    path: '/history',
    name: 'history',
    meta: { title: 'history.title' },
    component: HistoryView
  },
  {
    path: '/settings',
    name: 'settings',
    meta: { title: 'settings.title' },
    component: SettingsView
  },
  {
    path: '/',
    redirect: HISTORY_TASK_ROUTE_PATH[HistoryTaskType.ModelTraining]
  },
  {
    path: '/:pathMatch(.*)*',
    name: 'not-found',
    meta: { title: 'common.notFound.title' },
    component: NotFoundView
  }
];

const router = createRouter({
  history: createWebHashHistory(),
  routes: appRoutes
});

export default router;
