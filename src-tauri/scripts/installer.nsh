; Custom NSIS hooks for preserving shortcuts during upgrades
; Key: Skip uninstallation during upgrade to preserve shortcuts

!macro customHeader
  ; Skip uninstallation during upgrade - this is the key setting
  ; that prevents the "uninstall then install" behavior
  !define TAURI_SKIP_UNINSTALL_BEFORE_UPDATE

  ; Don't remove shortcuts during uninstall if it's an upgrade
  !define MUI_STARTMENUPAGE_NOREMOVE
!macroend

!macro customInstall
  ; Clean up old frontend assets before installing new version
  ; This prevents old JS/CSS files with different hashes from accumulating
  IfFileExists "$INSTDIR" 0 freshInstall
    ; Existing installation - clean assets directory first
    IfFileExists "$INSTDIR\assets" 0 skipAssetsCleanup
      RMDir /r "$INSTDIR\assets"
    skipAssetsCleanup:
    ; Also clean root level JS/CSS files (if any)
    Delete "$INSTDIR\*.js"
    Delete "$INSTDIR\*.css"
    Goto done

  freshInstall:
    ; Fresh install - create shortcuts
    CreateShortCut "$DESKTOP\Kafka Manager.lnk" "$INSTDIR\Kafka Manager.exe"
    CreateDirectory "$SMPROGRAMS\Kafka Manager"
    CreateShortCut "$SMPROGRAMS\Kafka Manager\Kafka Manager.lnk" "$INSTDIR\Kafka Manager.exe"

  done:
!macroend

!macro customUnInstall
  ; Only remove shortcuts on genuine uninstall (not during upgrade)
  Delete "$DESKTOP\Kafka Manager.lnk"
  Delete "$SMPROGRAMS\Kafka Manager\Kafka Manager.lnk"
  RMDir "$SMPROGRAMS\Kafka Manager"
!macroend

!macro customUnInstallPrev
  ; Called before installing new version during upgrade
  ; With TAURI_SKIP_UNINSTALL_BEFORE_UPDATE defined, this won't be called
!macroend