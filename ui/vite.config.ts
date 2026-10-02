import tailwindcss from '@tailwindcss/vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  base: './',
  plugins: [tailwindcss(), svelte()],
  server: {
    host: '127.0.0.1',
    proxy: {
      '/api': 'http://127.0.0.1:3000',
    },
  },
})
