; src-tauri/windows/installer-hooks.nsh
; NSIS hooks for the FileIT setup wizard (bundle.windows.nsis.installerHooks).
; After the files are copied, the wizard offers to install Indra OCR with
; resources\install-indra.ps1 (Python environment + models, ~230 MB download).
; Saved as UTF-8 with BOM so the Czech texts survive.

!macro NSIS_HOOK_POSTINSTALL
  ; Silent installs (/S) and updates never ask; run the script manually then.
  IfSilent fileit_indra_done
  ${If} $UpdateMode = 1
    Goto fileit_indra_done
  ${EndIf}
  IfFileExists "$LOCALAPPDATA\FileIT\indra-ocr\Scripts\indra-ocr.exe" fileit_indra_done

  MessageBox MB_YESNO|MB_ICONQUESTION "Nainstalovat Indra OCR pro čtení skenovaných dokumentů?$\r$\n$\r$\nBez OCR FileIT třídí skenované PDF a fotky jen podle názvu a složky.$\r$\n$\r$\nStáhne se asi 230 MB (Python prostředí a modely), potrvá to několik minut.$\r$\nPokud chybí Python, nainstaluje se Python 3.12 pro tohoto uživatele.$\r$\nVše zůstává na tomto počítači." /SD IDNO IDNO fileit_indra_done

  DetailPrint "Instaluji Indra OCR (může to trvat několik minut)..."
  nsExec::ExecToLog '"$WINDIR\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\resources\install-indra.ps1"'
  Pop $0
  StrCmp $0 "0" fileit_indra_ok

  MessageBox MB_OK|MB_ICONEXCLAMATION "Indra OCR se nepodařilo nainstalovat (kód $0).$\r$\n$\r$\nFileIT funguje i bez něj. Podrobnosti najdete v souboru:$\r$\n$LOCALAPPDATA\FileIT\indra-install.log$\r$\n$\r$\nInstalaci můžete později spustit znovu:$\r$\n$INSTDIR\resources\install-indra.ps1"
  Goto fileit_indra_done

  fileit_indra_ok:
  DetailPrint "Indra OCR je nainstalováno."

  fileit_indra_done:
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Updates reinstall over the old version: keep Indra.
  ${If} $UpdateMode <> 1
    IfSilent fileit_unindra_done
    IfFileExists "$LOCALAPPDATA\FileIT\indra-ocr\*.*" 0 fileit_unindra_done
    MessageBox MB_YESNO|MB_ICONQUESTION "Odebrat také Indra OCR (asi 230 MB)?" /SD IDNO IDNO fileit_unindra_done
    nsExec::ExecToLog '"$WINDIR\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\resources\install-indra.ps1" -Uninstall'
    Pop $0
    fileit_unindra_done:
  ${EndIf}
!macroend
