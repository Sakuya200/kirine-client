Function InstallBundledFfmpeg
  StrCpy $0 "$INSTDIR\lib\ffmpeg-8.0.1-essentials_build"
  StrCpy $1 "$0\bin\ffmpeg.exe"
  StrCpy $2 "$INSTDIR\resources\ffmpeg-8.0.1-essentials_build.zip"

  DetailPrint "Checking FFmpeg availability..."
  IfFileExists "$1" ffmpeg_ready 0

  StrCpy $0 '"$SYSDIR\cmd.exe" /C "where ffmpeg >nul 2>nul"'
  Call RunHiddenCommandWait
  StrCpy $3 "$0"
  ${If} $3 == 0
    DetailPrint "FFmpeg already exists in PATH. Skipping bundled extraction."
    Goto ffmpeg_cleanup
  ${EndIf}

  IfFileExists "$2" 0 ffmpeg_missing_archive
  DetailPrint "Extracting bundled FFmpeg to $INSTDIR\lib"
  Push "$2"
  Push "$INSTDIR\lib"
  Call ExtractBundledZipArchive
  Pop $3
  ${If} $3 != 0
    MessageBox MB_ICONEXCLAMATION "FFmpeg extraction failed with exit code $3. Some audio features may not work correctly."
    Goto ffmpeg_cleanup
  ${EndIf}

  IfFileExists "$1" ffmpeg_ready 0
  MessageBox MB_ICONEXCLAMATION "FFmpeg archive extraction completed, but ffmpeg.exe was not found in $0\bin."
  Goto ffmpeg_cleanup

ffmpeg_missing_archive:
  MessageBox MB_ICONEXCLAMATION "Bundled FFmpeg archive was not found: $2"
  Goto ffmpeg_done

ffmpeg_ready:
  Push "$0\bin"
  Call AddToUserPathIfMissing

ffmpeg_cleanup:
  IfFileExists "$2" 0 ffmpeg_done
  Delete "$2"

ffmpeg_done:
FunctionEnd