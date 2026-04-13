Function InstallBundledSox
  StrCpy $0 "$INSTDIR\lib\sox"
  StrCpy $1 "$0\sox.exe"
  StrCpy $2 "$INSTDIR\resources\sox-14.4.2-win32.exe"

  DetailPrint "Checking SoX availability..."
  IfFileExists "$1" sox_ready 0

  StrCpy $0 '"$SYSDIR\cmd.exe" /C "where sox >nul 2>nul"'
  Call RunHiddenCommandWait
  StrCpy $3 "$0"
  ${If} $3 == 0
    DetailPrint "SoX already exists in PATH. Skipping bundled installation."
    Goto sox_cleanup
  ${EndIf}

  IfFileExists "$2" 0 sox_missing_installer
  CreateDirectory "$0"
  DetailPrint "Installing bundled SoX to $0"
  StrCpy $0 '"$2" /S /D=$0'
  Call RunHiddenCommandWait
  StrCpy $3 "$0"
  ${If} $3 != 0
    MessageBox MB_ICONEXCLAMATION "SoX installation failed with exit code $3. Some audio features may not work correctly."
    Goto sox_cleanup
  ${EndIf}

  IfFileExists "$1" sox_ready 0
  MessageBox MB_ICONEXCLAMATION "SoX installer completed, but sox.exe was not found in $0."
  Goto sox_cleanup

sox_missing_installer:
  MessageBox MB_ICONEXCLAMATION "Bundled SoX installer was not found: $2"
  Goto sox_done

sox_ready:
  Push "$0"
  Call AddToUserPathIfMissing

sox_cleanup:
  IfFileExists "$2" 0 sox_done
  Delete "$2"

sox_done:
FunctionEnd