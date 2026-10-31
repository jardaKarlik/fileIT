// src/App.tsx
// Root component. Renders the top nav and routes between screens.


import { useStore } from './store';
import { Home }         from './components/Home';
import { HowItWorks }   from './components/HowItWorks';
import { SourcePath }   from './components/SourcePath';
import { FileTypes }    from './components/FileTypes';
import { Scanning }     from './components/Scanning';
import { Dashboard }    from './components/Dashboard';
import { Restructure }  from './components/Restructure';
import { Progress }     from './components/Progress';
import { Confirmation } from './components/Confirmation';
import { Ucebna }       from './components/Ucebna';
import './styles/global.css';

// Step index for the nav progress dots (null = hide dots)
const SCREEN_STEP: Record<string, number | null> = {
  home:         null,
  how_it_works: null,
  source_path:  null,
  file_types:   1,
  scanning:     2,
  dashboard:    3,
  restructure:  4,
  progress:     null,
  confirmation: null,
  ucebna:       null,
};

const SCREEN_LABEL: Record<string, string> = {
  home:         'Domů',
  how_it_works: 'Jak to funguje',
  source_path:  'Zdroj skenování',
  file_types:   'Typy souborů',
  scanning:     'Prohlížení…',
  dashboard:    'Přehled',
  restructure:  'Uspořádání',
  progress:     '',
  confirmation: '',
  ucebna:       'Učebna',
};

export default function App() {
  const { screen, setScreen } = useStore();

  const stepIdx = SCREEN_STEP[screen] ?? null;
  const hideSteps = stepIdx === null;

  return (
    <div id="app">
      {/* ── Top navigation ─────────────────────────────────────────────── */}
      <nav className={`topnav ${hideSteps ? 'home-mode' : ''}`}>
        <div className="logo" onClick={() => setScreen('home')} role="button" tabIndex={0}
          onKeyDown={e => e.key === 'Enter' && setScreen('home')}
        >
          <div className="logo-mark">
            <svg viewBox="0 0 18 18" fill="none" width="18" height="18">
              <path d="M3 14V8C3 7.4 3.4 7 4 7H6V5C6 4.4 6.4 4 7 4H11C11.6 4 12 4.4 12 5V7H14C14.6 7 15 7.4 15 8V14C15 14.6 14.6 15 14 15H4C3.4 15 3 14.6 3 14Z"
                fill="white" fillOpacity="0.95"/>
              <path d="M7 10H11M9 8V12" stroke="#4B0082" strokeWidth="1.8" strokeLinecap="round"/>
            </svg>
          </div>
          <span className="logo-text">File<span>IT</span></span>
        </div>

        <div className="nav-steps">
          {[1,2,3,4].map(i => (
            <div
              key={i}
              className={`nav-step ${
                stepIdx !== null && i < stepIdx  ? 'done'   :
                stepIdx !== null && i === stepIdx ? 'active' : ''
              }`}
            />
          ))}
        </div>

        <div style={{ width: 140, textAlign: 'right' }}>
          <span className="nav-label">{SCREEN_LABEL[screen] ?? ''}</span>
        </div>
      </nav>

      {/* ── Screen container ────────────────────────────────────────────── */}
      <main className="screen-container">
        {screen === 'home'         && <Home />}
        {screen === 'how_it_works' && <HowItWorks />}
        {screen === 'source_path'  && <SourcePath />}
        {screen === 'file_types'   && <FileTypes />}
        {screen === 'scanning'     && <Scanning />}
        {screen === 'dashboard'    && <Dashboard />}
        {screen === 'restructure'  && <Restructure />}
        {screen === 'progress'     && <Progress />}
        {screen === 'confirmation' && <Confirmation />}
        {screen === 'ucebna'       && <Ucebna />}
      </main>
    </div>
  );
}
