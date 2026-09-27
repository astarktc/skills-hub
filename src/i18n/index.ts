import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import { resources } from './resources'

void i18n.use(initReactI18next).init({
  resources,
  // EN only (BACKLOG #61). A stored "skills-language" value from the retired
  // switcher is deliberately never read.
  lng: 'en',
  fallbackLng: 'en',
  interpolation: {
    escapeValue: false,
  },
})

export default i18n
