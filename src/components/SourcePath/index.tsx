import { useState, useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useStore } from "../../store";
import "./SourcePath.css";

export function SourcePath() {
  const [selectedPath, setSelectedPath] = useState<string>("");
  const { setLastSourcePath } = useStore();

  useEffect(() => {
    // Load last selected path from localStorage on mount
    const saved = localStorage.getItem("fileIT_lastSourcePath");
    if (saved) {
      setSelectedPath(saved);
    }
  }, []);

  const handleBrowse = async () => {
    const selected = await open({
      directory: true,
      title: "Select folder to scan",
      defaultPath: selectedPath || undefined,
    });

    if (selected) {
      setSelectedPath(selected);
      localStorage.setItem("fileIT_lastSourcePath", selected);
    }
  };

  const handleStart = () => {
    if (selectedPath) {
      setLastSourcePath(selectedPath);
      // Navigation to FileTypes will happen in App.tsx routing
      window.location.hash = "#file-types";
    }
  };

  const handleUseDefault = () => {
    setSelectedPath("");
    localStorage.removeItem("fileIT_lastSourcePath");
    window.location.hash = "#file-types";
  };

  return (
    <div className="source-path-container">
      <div className="source-path-header">
        <h1>Vybrat zdroj skenování</h1>
        <p>Zvolte složku pro analýzu nebo nechte automatické detekci</p>
      </div>

      <div className="source-path-content">
        <div className="path-display">
          <input
            type="text"
            value={selectedPath || "Automatické detekování"}
            disabled
            className="path-input"
          />
          <button onClick={handleBrowse} className="btn-browse">
            📁 Procházet...
          </button>
        </div>

        <div className="path-info">
          {selectedPath && (
            <>
              <p className="selected-path">
                <strong>Vybraná cesta:</strong> {selectedPath}
              </p>
              <p className="path-size-hint">
                Skenování menší složky bude rychlejší pro testování
              </p>
            </>
          )}
          {!selectedPath && (
            <p className="auto-detect-hint">
              🔍 Automaticky se budou skenovat: OneDrive/Documents, Desktop, Downloads
            </p>
          )}
        </div>

        <div className="path-buttons">
          <button
            onClick={handleStart}
            className={`btn-start ${selectedPath ? "custom" : "disabled"}`}
          >
            {selectedPath ? "Pokračovat se zvolenou složkou" : "Začít automatické skenování"}
          </button>
          {selectedPath && (
            <button onClick={handleUseDefault} className="btn-secondary">
              Zrušit výběr
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
