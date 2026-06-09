// src/components/Home/index.tsx
import { useEffect } from 'react';
import { useStore } from '../../store';
import { api } from '../../utils/tauriApi';
import './Home.css';

export function Home() {
  const { setScreen, lastRunTimestamp, lastRunFileCount, setDetectedClouds, setCompletedRun, setLastRun } = useStore();

  // eslint-disable-next-line


  useEffect(() => {
    // On mount: init app state (check for pending confirmation, detect clouds)
    api.appInit().then((run) => {
      if (run) {
        setCompletedRun(run);
        if (run.confirmation_status === 'pending') {
          setScreen('confirmation');
          return;
        }
      }
      // Store last run info for display
      if (run) setLastRun(run.completed_at, run.files_moved);
    });

    api.detectClouds().then(setDetectedClouds);
  }, []);


  function goAnalyze() {
    setScreen('file_types');
  }

  function goRestructure() {
    setScreen('restructure');
  }

  function formatDate(iso: string) {
    return new Date(iso).toLocaleDateString('cs-CZ', { day: 'numeric', month: 'long', year: 'numeric' });
  }

  function relativeTime(iso: string) {
    const diff = Date.now() - new Date(iso).getTime();
    const days = Math.floor(diff / 86_400_000);
    if (days === 0) return 'dnes';
    if (days === 1) return 'včera';
    if (days < 7) return `před ${days} dny`;
    if (days < 30) return `před ${Math.floor(days / 7)} týdny`;
    return `před ${Math.floor(days / 30)} měsíci`;
  }

  return (
    <div className="home-wrap">
      <span className="corner-tr">FILEIT · V1.0 · HOME</span>
      <span className="corner-bl">N°00 · home</span>

      <div className="home-head">
        <div className="home-head-main">
          <div className="home-eyebrow">
            <span className="home-eyebrow-line" />
            <span>Vítejte zpět</span>
          </div>
          <h1 className="home-title">
            Co budeme dělat <em>dnes?</em>
          </h1>
          <p className="home-sub">
            Vyberte, jak vám má FileIT pomoci. Kdykoli během práce se můžete podívat, jak to funguje.
          </p>
        </div>

        {lastRunTimestamp && (
          <div className="lastrun-card">
            <div className="lastrun-label">Poslední spuštění</div>
            <div className="lastrun-date">{formatDate(lastRunTimestamp)}</div>
            {lastRunFileCount != null && (
              <div className="lastrun-meta">Uspořádáno {lastRunFileCount} souborů</div>
            )}
            <div className="lastrun-ago">{relativeTime(lastRunTimestamp)}</div>
          </div>
        )}
      </div>

      <div className="home-ctas">
        <div className="home-cta analyze" onClick={goAnalyze} role="button" tabIndex={0}>
          <div className="home-cta-inner">
            <div className="home-cta-row">
              <div className="home-cta-icon indigo">🔍</div>
              <div className="home-cta-mode indigo">Režim 1</div>
            </div>
            <div className="home-cta-title">Jen analyzovat<br />moje soubory</div>
            <p className="home-cta-desc">
              FileIT prohlédne vaše disky a ukáže vám přehled — co kde máte, kolik toho je, kdo jsou vaši klienti. Nic se neuspořádá ani nepřesune.
            </p>
            <div className="home-cta-actions">
              <button className="btn-indigo" onClick={(e) => { e.stopPropagation(); goAnalyze(); }}>
                Začít analýzu <span>→</span>
              </button>
              <span className="home-cta-time">~2–5 min</span>
            </div>
          </div>
        </div>

        <div className="home-cta restruct" onClick={goRestructure} role="button" tabIndex={0}>
          <div className="home-cta-badge">Hlavní</div>
          <div className="home-cta-inner">
            <div className="home-cta-row">
              <div className="home-cta-icon pink">✨</div>
              <div className="home-cta-mode pink">Režim 2</div>
            </div>
            <div className="home-cta-title">Uspořádat<br />moje soubory</div>
            <p className="home-cta-desc">
              FileIT prohlédne, porozumí a roztřídí vaše soubory do smysluplných složek. Před jakýmkoli přesunem uvidíte náhled a potvrdíte.
            </p>
            <div className="home-cta-actions">
              <button className="btn-run" onClick={(e) => { e.stopPropagation(); goRestructure(); }}>
                Spustit uspořádání <span>→</span>
              </button>
              <span className="home-cta-time">~5–10 min</span>
            </div>
          </div>
        </div>
      </div>

      <div className="home-how" onClick={() => setScreen('how_it_works')} role="button" tabIndex={0}>
        <div className="home-how-icon">ℹ</div>
        <div className="home-how-text">
          <div className="home-how-title">Jak to funguje</div>
          <div className="home-how-sub">Tři způsoby, jak FileIT uspořádává vaše soubory — lokálně, lokálně + cloud, z cloudu do cloudu</div>
        </div>
        <span className="home-how-link">Ukázat <span>→</span></span>
      </div>

      <div className="home-status">
        <div className="home-status-left">
          <span className="status-pulse">
            <span className="status-dot" />
            FileIT běží
          </span>
          <span>·</span>
          <span>Disky připravené</span>
        </div>
        <div className="home-status-right">
          <span className="home-status-link" onClick={() => setScreen('how_it_works')}>Nápověda</span>
        </div>
      </div>


    </div>
  );
}
