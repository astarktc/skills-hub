import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import './i18n'
import App from './App.tsx'

const render = () =>
  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  )

// Dev-only fixture mode (`npm run dev:fixture`): install the in-memory
// backend before the first render. The condition is a compile-time constant,
// so a production build keeps only the direct `render()`.
if (import.meta.env.VITE_MOCK_BACKEND === '1') {
  void import('./fixtures').then(({ installFixtureBackend }) => {
    installFixtureBackend()
    render()
  })
} else {
  render()
}
