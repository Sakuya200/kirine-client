import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': '/src'
    }
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 3333,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 3333
        }
      : undefined,
    watch: {
      // 3. tell vite to ignore watching `src-tauri` and `src-model`
      //    `src-model` contains Python venvs and downloaded model weights that can
      //    create tens of thousands of files during runtime initialisation, causing
      //    the chokidar watcher to exhaust the Node.js / V8 heap (OOM / Full GC).
      ignored: ['**/src-tauri/**', '**/src-model/**']
    }
  }
}));
