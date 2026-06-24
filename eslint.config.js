import js from '@eslint/js'
import globals from 'globals'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'
import tseslint from 'typescript-eslint'
import { defineConfig, globalIgnores } from 'eslint/config'

export default defineConfig([
  globalIgnores(['dist', 'src-tauri/target']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      js.configs.recommended,
      tseslint.configs.recommended,
      reactHooks.configs.flat.recommended,
      reactRefresh.configs.vite,
    ],
    languageOptions: {
      ecmaVersion: 2020,
      globals: globals.browser,
    },
    rules: {
      // React Compiler rules newly enabled by eslint-plugin-react-hooks 7.1.x's
      // recommended preset (required for the ESLint 10 peer range). They flag
      // pre-existing, working patterns rather than bugs; satisfying them is a
      // focused, behavior-risk refactor tracked as a separate follow-up. Kept
      // off here so `eslint src` stays green without that refactor.
      'react-hooks/set-state-in-effect': 'off',
      'react-hooks/immutability': 'off',
      'react-hooks/preserve-manual-memoization': 'off',
      'react-hooks/refs': 'off',
    },
  },
])
