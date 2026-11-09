import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import '../styles.css'
import Popup from './Popup'

// 일반 브라우저에서 화면만 확인할 때 (개발 전용)
if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) await import('../shared/devMock')

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Popup />
  </StrictMode>,
)
