Function InstallBundledSox
  ; sox 解压到 $INSTDIR\lib\sox-14-4-2（连字符，canonical 目录名；资源 zip 顶层为
  ; sox-14.4.2，解压后需重命名）。不探测系统 sox、不注册系统 PATH：运行时不按 PATH
  ; 调用 sox（预留依赖），如未来启用由 Rust 运行时注入进程级 PATH 前缀解析
  ; （src-tauri/src/utils/process.rs 的 bundled_tool_path_prefix），恒保证自带副本。
  StrCpy $4 "$INSTDIR\lib\sox-14-4-2"
  StrCpy $1 "$4\sox.exe"
  StrCpy $2 "$INSTDIR\resources\sox-14.4.2-win32.zip"

  IfFileExists "$1" sox_cleanup 0

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

  ; 规整 zip 顶层目录名 sox-14.4.2 -> sox-14-4-2
  IfFileExists "$INSTDIR\lib\sox-14.4.2" 0 sox_check_exe
  Rename "$INSTDIR\lib\sox-14.4.2" "$4"

sox_check_exe:
  IfFileExists "$1" 0 sox_exe_missing

sox_cleanup:
  IfFileExists "$2" 0 sox_done
  Delete "$2"
  Goto sox_done

sox_exe_missing:
  MessageBox MB_ICONEXCLAMATION "SoX archive extraction completed, but sox.exe was not found in $4."
  Goto sox_done

sox_missing_archive:
  MessageBox MB_ICONEXCLAMATION "Bundled SoX archive was not found: $2"

sox_done:
FunctionEnd
