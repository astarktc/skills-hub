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
  if (new URLSearchParams(window.location.search).has('specimen')) {
    // `/?specimen`: the primitives specimen (DESIGN.md's measuring page).
    void import('./components/ui/specimen').then(({ default: Specimen }) =>
      createRoot(document.getElementById('root')!).render(
        <StrictMode>
          <Specimen />
        </StrictMode>,
      ),
    )
  } else if (new URLSearchParams(window.location.search).get('proto') === 'harnesses') {
    // PROTOTYPE (round 20 Wave B, throwaway branch only): `/?proto=harnesses&variant=a|b|c`.
    void import('./components/settings/prototype/HarnessesPrototype').then(({ default: HarnessesPrototype }) =>
      createRoot(document.getElementById('root')!).render(
        <StrictMode>
          <HarnessesPrototype />
        </StrictMode>,
      ),
    )
  } else {
    void import('./fixtures').then(({ installFixtureBackend }) => {
      installFixtureBackend()
      render()
    })
  }
} else {
  render()
}
