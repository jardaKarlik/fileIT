// src/components/HowItWorks/index.tsx
import React, { useState } from 'react';
import { useStore } from '../../store';
import './HowItWorks.css';

type TabKey = 'local' | 'cloud' | 'c2c';

const TABS: { key: TabKey; icon: string; name: string; cls: string }[] = [
  { key: 'local', icon: '💻', name: 'Lokálně',          cls: 'indigo' },
  { key: 'cloud', icon: '☁',  name: 'Lokálně + cloud', cls: 'pink'   },
  { key: 'c2c',   icon: '⇄',  name: 'Z cloudu do cloudu', cls: 'coral' },
];

type FlowStep = { icon: string; cls: string; title: string; body: string; path?: string; connector?: string };

const FLOWS: Record<TabKey, FlowStep[]> = {
  local: [
    { icon:'📁', cls:'start',     title:'Vaše stávající soubory',     body:'Rozházené po složkách, ploše, ve Staženém', connector:'plain' },
    { icon:'⚙',  cls:'engine',   title:'FileIT analyzuje a uspořádá', body:'Přečte obsah, rozpozná klienty a instituce', connector:'plain' },
    { icon:'📂', cls:'end-local', title:'Uspořádané soubory na disku', body:'Smysluplná struktura — podle klienta, data, instituce', path:'C:\\FileIT\\Organized\\' },
  ],
  cloud: [
    { icon:'📁', cls:'start',    title:'Vaše stávající soubory',       body:'Místně na počítači, kdekoli je máte', connector:'plain' },
    { icon:'⚙',  cls:'engine',  title:'FileIT uspořádá do cloudové složky', body:'Výsledek zapíše do složky, kterou sleduje váš cloud klient', connector:'plain' },
    { icon:'📂', cls:'end-local',title:'Uspořádané soubory na disku',  body:'Ve složce cloudového klienta', path:'…\\OneDrive\\FileIT\\Organized\\', connector:'pink' },
    { icon:'☁',  cls:'sync',    title:'Cloud klient zasynchronizuje', body:'Automaticky — FileIT do toho nijak nezasahuje' },
  ],
  c2c: [
    { icon:'☁',  cls:'cloud-src', title:'Soubory ve zdrojovém cloudu',  body:'Např. osobní OneDrive, synchronizovaný na počítač', path:'…\\OneDrive\\', connector:'plain' },
    { icon:'⚙',  cls:'engine',   title:'FileIT přečte a uspořádá',     body:'Pracuje lokálně s tím, co sync klient zdrojového cloudu stáhl', connector:'plain' },
    { icon:'📂', cls:'end-cloud', title:'Zapíše do cílového cloudu',    body:'Do složky druhého cloud klienta', path:'…\\Google Drive\\FileIT\\Organized\\', connector:'coral' },
    { icon:'☁',  cls:'sync-coral',title:'Cílový cloud zasynchronizuje', body:'Vše se objeví ve druhém cloudu automaticky' },
  ],
};

const FACTS: Record<TabKey, { label: string; title: string; sub: string }[]> = {
  local: [
    { label:'Stačí',      title:'Aplikace FileIT',          sub:'A volná cílová složka' },
    { label:'Bezpečnost', title:'Nic neopouští počítač',    sub:'Včetně analýzy obsahu' },
    { label:'Záloha',     title:'Safe-side ZIP',            sub:'Obnova jedním klikem' },
  ],
  cloud: [
    { label:'Potřebujete', title:'Běžící cloud klient',      sub:'OneDrive, Google Drive, Dropbox, Box' },
    { label:'Přístup',     title:'Kdekoli se přihlásíte',   sub:'Telefon, tablet, jiný počítač' },
    { label:'Kopie',       title:'Jedna — na disku',         sub:'Cloud ji sám přenese' },
  ],
  c2c: [
    { label:'Potřebujete', title:'Dva běžící cloudy',        sub:'Zdrojový a cílový, oba synchronizované' },
    { label:'Omezení',     title:'Limity cloudů',            sub:'Každý má jiná pravidla velikosti' },
    { label:'Výhoda',      title:'Žádné API',                sub:'Žádné přihlášení do cloudu' },
  ],
};

const DID_YOU_KNOW: Record<TabKey, string> = {
  local: 'i v lokálním režimu si FileIT vytvoří bezpečnostní zálohu před jakýmkoli přesunem. Pokud by se cokoli nepovedlo, vrátíme vše na původní místo jedním klikem.',
  cloud: 'FileIT nepoužívá žádné cloudové API ani přihlašovací údaje. Spoléhá se na to, že váš cloud klient už umí synchronizovat — a udělá to sám, jakmile se objeví nové soubory ve sledované složce.',
  c2c:   'zdrojový cloud se nevymaže. FileIT originál nechá na místě, jen z něj vytvoří uspořádanou kopii v cílovém cloudu. Můžete si sami rozhodnout, kdy (a jestli vůbec) původní soubory smažete.',
};

const WHEN: Record<TabKey, { icon: string; label: string; text: string }> = {
  local: { icon:'🔒', label:'Kdy tento režim', text:'Když pracujete převážně lokálně, nemáte cloud, nebo jsou vaše soubory citlivé a nechcete je nikam posílat.' },
  cloud: { icon:'☁',  label:'Kdy tento režim', text:'Když chcete mít soubory přístupné i mimo počítač — z telefonu, tabletu, nebo pro sdílení s kolegy. Cloud klient musí být spuštěný a aktivní.' },
  c2c:   { icon:'⇄',  label:'Kdy tento režim', text:'Když se vám soubory omylem ocitly v jiném cloudu, než měly. Nebo když stěhujete práci mezi osobním a firemním úložištěm.' },
};

const STORY: Record<TabKey, { headline: string; body: string }> = {
  local: { headline:'Všechno zůstává\nna vašem počítači.', body:'FileIT přečte vaše soubory, porozumí jejich obsahu a uspořádá je do smysluplných složek — vše na vašem disku. Žádný cloud, žádné nahrávání, nic neopustí váš počítač.' },
  cloud: { headline:'Uspořádáno a\nautomaticky v cloudu.', body:'FileIT uspořádá vaše soubory do složky, kterou sleduje váš cloudový klient — OneDrive, Google Drive, Dropbox nebo Box. Ten se postará o nahrávání do cloudu.' },
  c2c:   { headline:'Z jednoho cloudu\ndo druhého.', body:'Máte soubory na osobním OneDrive, ale patří do firemního Google Drive? FileIT přečte ze zdrojového cloudu, uspořádá, a výsledek uloží do cílového cloudu.' },
};

export function HowItWorks() {
  const { setScreen } = useStore();
  const [active, setActive] = useState<TabKey>('local');

  const tab = active;
  const story = STORY[tab];
  const when = WHEN[tab];
  const flows = FLOWS[tab];
  const facts = FACTS[tab];
  const dyk = DID_YOU_KNOW[tab];

  return (
    <div className="how-wrap">
      <button className="how-back" onClick={() => setScreen('home')}>← Zpět na domů</button>
      <span className="corner-tr">FILEIT · V1.0 · HOW</span>
      <span className="corner-bl">N°06 · jak to funguje</span>

      <div className="how-eyebrow">Nápověda</div>
      <h2 className="how-title">Jak to funguje</h2>
      <p className="how-sub">FileIT umí uspořádat vaše soubory třemi způsoby. Který se hodí vám, záleží na tom, kde jsou dnes a kam je chcete dostat.</p>

      {/* Tabs */}
      <div className="how-tabs">
        {TABS.map(t => (
          <button key={t.key} className={`how-tab ${active === t.key ? 'active' : ''}`} onClick={() => setActive(t.key)}>
            <div className={`how-tab-icon ${t.cls}`}>{t.icon}</div>
            <span className="how-tab-name">{t.name}</span>
            {active === t.key && <span className={`how-tab-badge ${t.cls}`}>Zvoleno</span>}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="how-grid">
        {/* Left: story */}
        <div>
          <div className="how-story-title">{story.headline.split('\n').map((l,i) => <React.Fragment key={i}>{l}{i < 1 && <br />}</React.Fragment>)}</div>
          <p className="how-story-body">{story.body}</p>
          <div className={`how-when ${tab}`}>
            <span className="how-when-icon">{when.icon}</span>
            <div>
              <div className={`how-when-label ${tab}`}>{when.label}</div>
              <div className="how-when-text">{when.text}</div>
            </div>
          </div>
        </div>

        {/* Right: flow diagram */}
        <div className="how-flow">
          <div className="how-flow-label">Tok souborů</div>
          {flows.map((step, i) => (
            <div key={i} className="flow-step">
              <div className="flow-step-icon-col">
                <div className={`flow-step-icon ${step.cls}`}>{step.icon}</div>
                {i < flows.length - 1 && <div className={`flow-step-connector ${step.connector ?? ''}`} />}
              </div>
              <div className="flow-step-text">
                <div className="flow-step-title">{step.title}</div>
                <div className="flow-step-body">{step.body}</div>
                {step.path && <div className="flow-step-path">{step.path}</div>}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Fact cards */}
      <div className="how-facts">
        {facts.map(f => (
          <div key={f.label} className="fact-card">
            <div className={`fact-label ${tab}`}>{f.label}</div>
            <div className="fact-title">{f.title}</div>
            <div className="fact-sub">{f.sub}</div>
          </div>
        ))}
      </div>

      {/* Did you know */}
      <div className={`did-you-know ${tab}`}>
        <span className="did-icon">💡</span>
        <div><span className="did-title">Víte, že…</span> {dyk}</div>
      </div>
    </div>
  );
}
