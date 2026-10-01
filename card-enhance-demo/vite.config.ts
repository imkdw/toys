import { defineConfig } from 'vite';

export default defineConfig({
  root: 'web',
  build: { outDir: '../dist/web', emptyOutDir: true },
  server: { proxy: { '/api': 'http://localhost:4000', '/files': 'http://localhost:4000' } },
});
