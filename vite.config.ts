import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  root: 'ui',
  plugins: [svelte()],
  clearScreen: false,
  resolve: { conditions: ['browser'] },
  server: { port: 1420, strictPort: true },
  build: { outDir: '../dist', emptyOutDir: true },
  test: { environment: 'jsdom', include: ['**/*.test.ts'], setupFiles: ['./test-setup.ts'] }
});
