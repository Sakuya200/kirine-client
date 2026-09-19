import { createApp } from 'vue';

import App from './App.vue';
import router from './routers';
import pinia from './stores';
import './assets/styles/tailwind.css';
import './assets/styles/theme.css';
import { i18n } from './locales';

const app = createApp(App);

app.use(pinia);
app.use(i18n);
app.use(router);
app.mount('#app');
