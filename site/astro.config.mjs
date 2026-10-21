import { defineConfig } from 'astro/config';

export default defineConfig({
  site: 'https://globlin.pages.dev',
  output: 'static',
  compressHTML: false,
  build: {
    format: 'directory',
    inlineStylesheets: 'always',
  },
});
