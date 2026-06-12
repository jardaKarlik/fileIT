// AnalysisWarningModal.tsx
// Warning modal for Mode 2 entry when no analysis or old analysis is available.

export interface AnalysisWarningModalProps {
  scenario: 'no_analysis' | 'old_analysis';
  analysisAgeText?: string; // e.g., "10 dní"
  onUseSampleData: () => void;
  onStartScan: () => void;
  onProceedAnyway?: () => void; // For old analysis
}

export function AnalysisWarningModal({
  scenario,
  analysisAgeText,
  onUseSampleData,
  onStartScan,
  onProceedAnyway,
}: AnalysisWarningModalProps) {
  const isOldAnalysis = scenario === 'old_analysis';

  return (
    <div className="modal-overlay" onClick={onUseSampleData}>
      <div className="modal-card analysis-warning" onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div className="modal-head">
          <div className="modal-icon warning">⚠️</div>
          <div>
            <div className="modal-eyebrow">
              {isOldAnalysis ? 'Upozornění: stará analýza' : 'Upozornění'}
            </div>
            <div className="modal-title">
              {isOldAnalysis
                ? `Vaše analýza je stará (${analysisAgeText})`
                : 'Chybí analýza souborů'}
            </div>
            <div className="modal-sub">
              {isOldAnalysis
                ? 'Uspořádání bez čerstvé analýzy může vést k nesprávné klasifikaci. Doporučujeme spustit nový sken.'
                : 'Uspořádání bez analýzy vyžaduje odborné znalosti a není doporučeno. Všichni finanční poradci by měli nejdříve spustit analýzu.'}
            </div>
          </div>
        </div>

        {/* Content */}
        <div className="modal-content analysis-warning-content">
          <div className="warning-box">
            <div className="warning-icon">💡</div>
            <div className="warning-text">
              <strong>Proč je analýza důležitá:</strong>
              <ul>
                <li>Identifikuje vaše klienty a jejich dokumenty</li>
                <li>Rozpoznává instituce a typy dokumentů</li>
                <li>Hlásí potenciální duplikáty a rizika</li>
                <li>Zajišťuje spolehlivé a bezpečné uspořádání</li>
              </ul>
            </div>
          </div>
        </div>

        {/* Actions */}
        <div className="modal-actions analysis-warning-actions">
          <button className="modal-btn-primary" onClick={onStartScan}>
            {isOldAnalysis ? '↺ Spustit nový sken' : '↺ Spustit analýzu'}
          </button>
          <div className="modal-actions-row">
            <button className="modal-btn-secondary" onClick={onUseSampleData}>
              {isOldAnalysis ? '← Vrátit se' : 'Použít vzorek (demo)'}
            </button>
            {isOldAnalysis && (
              <button className="modal-btn-tertiary" onClick={onProceedAnyway}>
                Pokračovat s starou analýzou
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
