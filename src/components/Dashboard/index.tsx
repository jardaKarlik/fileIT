// src/components/Dashboard/index.tsx

import { useStore } from '../../store';
import './Dashboard.css';


const MONTHS = ['dub','kvě','čvn','čvc','srp','zář','říj','lis','pro','led','úno','bře'];
const VALS   = [12,18,8,24,31,15,42,28,19,37,48,22];
const MAX    = Math.max(...VALS);

const CUSTOMERS = [
  ['Novák Pavel',22],['Horáková Jana',18],['Müller Thomas',15],
  ['Svoboda Lucie',12],['Kratochvíl P.',9],['Dvořák Martin',7],
] as [string,number][];

export function Dashboard() {
  const { setScreen, scanResult } = useStore();

  const totalFiles = scanResult?.total_files ?? 284;
  const totalGB   = ((scanResult?.total_size_bytes ?? 2_576_351_232) / 1e9).toFixed(1).replace('.', ',');
  const customers = scanResult?.customers_found ?? 38;
  const dupes     = scanResult?.duplicates_found ?? 24;

  // Count unclassified files for the Učebna badge
  const unclassifiedCount = scanResult?.files
    ? scanResult.files.filter(f => !f.institution || f.confidence < 0.65).length
    : 0;

  return (
    <div className="dash-screen">
      {/* Hero band */}
      <div className="dash-hero">
        <div className="dash-hero-inner">
          <div className="dash-hero-top">
            <div>
              <div className="dash-title">Přehled vašich souborů</div>
              <div className="dash-subtitle">Prohlédnuto {totalFiles} souborů · {new Date().toLocaleDateString('cs-CZ', { day:'numeric',month:'long',year:'numeric' })}</div>
            </div>
            <div className="dash-top-actions">
              {/* Učebna — bonus feature, outlined ghost, left-most */}
              <button className="btn-hero-ucebna" onClick={() => setScreen('ucebna')}>
                <span>✦</span> Učebna
                {unclassifiedCount > 0 && (
                  <span className="ucebna-badge">{unclassifiedCount}</span>
                )}
              </button>
              <button className="btn-hero-ghost" onClick={() => {}}>Exportovat relaci</button>
              <button className="btn-hero-primary" onClick={() => setScreen('scanning')}>
                <span>↺</span> Znovu prohlédnout
              </button>
            </div>
          </div>
          <div className="stats-row">
            {[
              { label: 'Celkem souborů', val: totalFiles, sub: 'ze 3 disků' },
              { label: 'Celková velikost', val: `${totalGB} GB`, sub: '~180 MB měsíčně' },
              { label: 'Nalezeno klientů', val: customers, sub: 'z obsahu souborů' },
              { label: 'Duplikáty', val: dupes, sub: 'jen informativně', warn: true },
            ].map(s => (
              <div key={s.label} className="stat-card-hero">
                <div className="stat-label-hero">{s.label}</div>
                <div className="stat-value-hero">{s.val}</div>
                <div className={`stat-sub-hero ${s.warn ? 'warn' : ''}`}>{s.sub}</div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Body */}
      <div className="dash-body">
        {/* CTAs */}
        <div className="hero-ctas">
          <div className="cta-card restructure" onClick={() => setScreen('restructure')}>
            <div className="cta-card-content">
              <div className="cta-card-label">Hlavní akce</div>
              <div className="cta-card-title">Uspořádat</div>
              <div className="cta-card-desc">Roztřídit {totalFiles} souborů do logických složek podle vašich pravidel.</div>
            </div>
            <div className="cta-arrow">→</div>
          </div>
          <div className="cta-card backup" onClick={() => {}}>
            <div className="cta-card-content">
              <div className="cta-card-label">Chraňte svou práci</div>
              <div className="cta-card-title">Zálohovat</div>
              <div className="cta-card-desc">Bezpečnostní ZIP + volitelně kopie na OneDrive, Google Drive nebo Dropbox.</div>
            </div>
            <div className="cta-arrow">→</div>
          </div>
        </div>

        {/* Charts row */}
        <div className="charts-row">
          <div className="chart-card">
            <div className="chart-title">Aktivita souborů v čase <span className="chart-tag">podle data vzniku</span></div>
            <div className="activity-bars">
              {VALS.map((v,i) => (
                <div key={i} className="act-bar-wrap">
                  <div className="act-bar" style={{
                    height: `${(v/MAX)*90}px`,
                    background: v===MAX ? 'linear-gradient(180deg,#FF1493,#FE865B)' : v>25 ? 'var(--pink-soft)' : '#E8D4E8'
                  }} title={`${MONTHS[i]}: ${v} souborů`} />
                  <div className="act-month">{MONTHS[i]}</div>
                </div>
              ))}
            </div>
          </div>

          <div className="chart-card">
            <div className="chart-title">Soubory podle klientů</div>
            {CUSTOMERS.map(([name, count]) => (
              <div key={name} className="cust-row">
                <div className="cust-name">{name}</div>
                <div className="cust-bar-wrap"><div className="cust-bar" style={{ width: `${Math.round(count/22*100)}%` }} /></div>
                <div className="cust-count">{count}</div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
