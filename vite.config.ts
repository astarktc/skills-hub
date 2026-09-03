/// <reference types="vitest/config" />
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    // Override with VITE_DEV_PORT when 5173 is taken; tauri:dev follows it (scripts/tauri-dev.mjs).
    port: Number(process.env.VITE_DEV_PORT ?? 5173),
    strictPort: true,
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.{ts,tsx}'],
  },
})
