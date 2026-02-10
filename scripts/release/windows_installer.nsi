!include "MUI2.nsh"

!ifndef OUTPUT_FILE
  !error "OUTPUT_FILE define is required"
!endif

!ifndef SOURCE_EXE
  !error "SOURCE_EXE define is required"
!endif

!ifndef UPDATER_EXE
  ; Optional: path to butterpaper-updater.exe to install alongside the main app.
!endif

!ifndef APP_NAME
  !define APP_NAME "ButterPaper"
!endif

!ifndef INSTALL_SUBDIR
  !define INSTALL_SUBDIR "ButterPaper"
!endif

Name "${APP_NAME}"
OutFile "${OUTPUT_FILE}"
InstallDir "$LOCALAPPDATA\Programs\${INSTALL_SUBDIR}"
RequestExecutionLevel user
Unicode True

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"
  File "/oname=ButterPaper.exe" "${SOURCE_EXE}"
  !ifdef UPDATER_EXE
    File "/oname=butterpaper-updater.exe" "${UPDATER_EXE}"
  !endif
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateDirectory "$SMPROGRAMS\${INSTALL_SUBDIR}"
  CreateShortCut "$SMPROGRAMS\${INSTALL_SUBDIR}\${APP_NAME}.lnk" "$INSTDIR\ButterPaper.exe"
  CreateShortCut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\ButterPaper.exe"
SectionEnd

Section "Uninstall"
  Delete "$DESKTOP\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${INSTALL_SUBDIR}\${APP_NAME}.lnk"
  RMDir "$SMPROGRAMS\${INSTALL_SUBDIR}"
  Delete "$INSTDIR\ButterPaper.exe"
  Delete "$INSTDIR\butterpaper-updater.exe"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
SectionEnd
