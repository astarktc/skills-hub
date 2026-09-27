/// <reference types="vitest/config" />
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig(({ mode }) => ({
  plugins: [react(), tailwindcss()],
  define: {
    // Fixture-backed browser mode (`npm run dev:fixture` = `--mode fixture`):
    // a literal, so every other mode (production included) folds the fixture
    // branches away and never bundles `src/fixtures/`.
    'import.meta.env.VITE_MOCK_BACKEND': JSON.stringify(mode === 'fixture' ? '1' : '0'),
  },
  server: {
    // Override with VITE_DEV_PORT when 5173 is taken; tauri:dev follows it (scripts/tauri-dev.mjs).
    port: Number(process.env.VITE_DEV_PORT ?? 5173),
    strictPort: true,
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.{ts,tsx}'],
  },
}))
