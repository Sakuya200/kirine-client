Function InstallBundledSox
  StrCpy $4 "$INSTDIR\lib\sox-14-4-2"
  StrCpy $1 "$4\sox.exe"
  StrCpy $2 "$INSTDIR\resources\sox-14.4.2-win32.zip"

  DetailPrint "Checking SoX availability..."
  IfFileExists "$1" sox_ready 0

  StrCpy $0 '"$SYSDIR\cmd.exe" /C "where sox >nul 2>nul"'
  Call RunHiddenCommandWait
  StrCpy $3 "$0"
  ${If} $3 == 0
    DetailPrint "SoX already exists in PATH. Skipping bundled extraction."
    Goto sox_cleanup
  ${EndIf}

  IfFileExists "$2" 0 sox_missing_archive
  DetailPrint "Extracting bundled SoX to $INSTDIR\lib"
  Push "$2"
  Push "$INSTDIR\lib"
  Call ExtractBundledZipArchive
  Pop $3
  ${If} $3 != 0
    MessageBox MB_ICONEXCLAMATION "SoX extraction failed with exit code $3. Some audio features may not work correctly."
    Goto sox_cleanup
  ${EndIf}

  IfFileExists "$1" sox_ready 0
  MessageBox MB_ICONEXCLAMATION "SoX archive extraction completed, but sox.exe was not found in $4."
  Goto sox_cleanup

sox_missing_archive:
  MessageBox MB_ICONEXCLAMATION "Bundled SoX archive was not found: $2"
  Goto sox_done

sox_ready:
  Push "$4"
  Call AddToUserPathIfMissing

sox_cleanup:
  IfFileExists "$2" 0 sox_done
  Delete "$2"

sox_done:
FunctionEnd