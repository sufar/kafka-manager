; Custom NSIS hooks for preserving shortcuts during upgrades
; Key: Skip uninstallation during upgrade to preserve shortcuts

!macro customHeader
  ; NSIS setting to allow overwriting existing installation
  ; without running the uninstaller first
  !define UNINSTALL_CMD "uninstall.exe"

  ; Set to allow overwriting files during upgrade
  SetOverwrite on
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
    ; Delete old uninstaller to prevent conflicts
    Delete "$INSTDIR\uninstall.exe"
    Goto done

  freshInstall:
    ; Fresh install - create shortcuts
    CreateShortCut "$DESKTOP\Kafka Manager.lnk" "$INSTDIR\Kafka Manager.exe"
    CreateDirectory "$SMPROGRAMS\Kafka Manager"
    CreateShortCut "$SMPROGRAMS\Kafka Manager\Kafka Manager.lnk" "$INSTDIR\Kafka Manager.exe"

  done:
    ; Always create shortcuts (for both fresh install and upgrade)
    CreateShortCut "$DESKTOP\Kafka Manager.lnk" "$INSTDIR\Kafka Manager.exe"
    CreateDirectory "$SMPROGRAMS\Kafka Manager"
    CreateShortCut "$SMPROGRAMS\Kafka Manager\Kafka Manager.lnk" "$INSTDIR\Kafka Manager.exe"
!macroend

!macro customUnInstall
  ; Remove shortcuts on uninstall
  Delete "$DESKTOP\Kafka Manager.lnk"
  Delete "$SMPROGRAMS\Kafka Manager\Kafka Manager.lnk"
  RMDir "$SMPROGRAMS\Kafka Manager"
!macroend

!macro customUnInstallPrev
  ; Called before installing new version during upgrade
  ; Delete old uninstaller to prevent full uninstall
  IfFileExists "$INSTDIR\uninstall.exe" 0 +2
    Delete "$INSTDIR\uninstall.exe"
!macroend