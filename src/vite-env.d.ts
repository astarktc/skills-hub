/// <reference types="vite/client" />

interface ImportMetaEnv {
  /**
   * `"1"` in fixture mode (`npm run dev:fixture`, i.e. `vite --mode fixture`):
   * the app runs against the in-memory backend in `src/fixtures/`. Defined as
   * a compile-time constant by `vite.config.ts` — `"0"` in every other mode.
   */
  readonly VITE_MOCK_BACKEND: "0" | "1";
}
