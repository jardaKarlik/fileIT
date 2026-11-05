// src/main.tsx
// Fonts bundled locally — no CDN, works fully offline inside Tauri
import '@fontsource/inter/300.css';
import '@fontsource/inter/400.css';
import '@fontsource/inter/500.css';
import '@fontsource/inter/600.css';
import '@fontsource/inter/700.css';
import '@fontsource/fraunces/400.css';
import '@fontsource/fraunces/600.css';
import '@fontsource/fraunces/700.css';
import '@fontsource/fraunces/800.css';
import '@fontsource-variable/jetbrains-mono';

import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { platformReady, WINDOWS_DEFAULTS } from './utils/platform';
import { useStore } from './store';

// Platform facts (default target folder, separators) are needed
// synchronously by several screens: load them before the first render.
platformReady.then((info) => {
  const { targetPath, setTargetPath } = useStore.getState();
  if (targetPath === WINDOWS_DEFAULTS.default_target_path && info.default_target_path !== targetPath) {
    setTargetPath(info.default_target_path);
  }
  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
});
