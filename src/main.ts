import { createApp } from 'vue';

import App from './App.vue';
import router from './routers';
import pinia from './stores';
import './assets/styles/tailwind.css';
import './assets/styles/theme.css';

const app = createApp(App);

app.use(pinia);
app.use(router);
app.mount('#app');
