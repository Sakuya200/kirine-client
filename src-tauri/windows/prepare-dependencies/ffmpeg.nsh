Function InstallBundledFfmpeg
  ; ffmpeg 解压到 src-model 同级目录（$INSTDIR\lib，与 src-model 并列），canonical
  ; 目录名 ffmpeg-8.1.2。zip 顶层目录为 ffmpeg-8.1.2-full_build-shared，解压后规整为
  ; ffmpeg-8.1.2。不再探测系统 ffmpeg：首次安装或 lib\ffmpeg-8.1.2 缺失即解压。
  StrCpy $4 "$INSTDIR\lib"
  StrCpy $5 "ffmpeg-8.1.2"
  StrCpy $6 "ffmpeg-8.1.2-full_build-shared"
  StrCpy $1 "$4\$5\bin\ffmpeg.exe"
  StrCpy $2 "$INSTDIR\resources\ffmpeg-8.1.2.zip"

  ; 清理旧版（8.0.1，原解压在 lib\）残留目录，并注销其在 PATH 中的条目。
  RMDir /r "$INSTDIR\lib\ffmpeg-8.0.1-essentials_build"
  Push "$INSTDIR\lib\ffmpeg-8.0.1-essentials_build\bin"
  Call RemoveFromUserPath
  ; 注销当前 bin 条目后再重新注册，避免遗留旧路径（更新前删除已注册条目）。
  Push "$4\$5\bin"
  Call RemoveFromUserPath

  ; 仅在 canonical 目录缺失时解压（首次安装或被删后重建）。
  IfFileExists "$4\$5" ffmpeg_register_path 0

  IfFileExists "$2" 0 ffmpeg_missing_archive
  DetailPrint "Extracting bundled FFmpeg to $4"
  CreateDirectory "$4"
  Push "$2"
  Push "$4"
  Call ExtractBundledZipArchive
  Pop $3
  ${If} $3 != 0
    MessageBox MB_ICONEXCLAMATION "FFmpeg extraction failed with exit code $3. Some audio features may not work correctly."
    Goto ffmpeg_cleanup
  ${EndIf}

  ; 规整 zip 顶层目录名 ffmpeg-8.1.2-full_build-shared -> ffmpeg-8.1.2
  IfFileExists "$4\$6" 0 ffmpeg_check_exe
  Rename "$4\$6" "$4\$5"

ffmpeg_check_exe:
  IfFileExists "$1" 0 ffmpeg_exe_missing

ffmpeg_register_path:
  Push "$4\$5\bin"
  Call AddToUserPathIfMissing
  Goto ffmpeg_cleanup

ffmpeg_exe_missing:
  MessageBox MB_ICONEXCLAMATION "FFmpeg archive extraction completed, but ffmpeg.exe was not found in $4\$5\bin."
  Goto ffmpeg_cleanup

ffmpeg_missing_archive:
  MessageBox MB_ICONEXCLAMATION "Bundled FFmpeg archive was not found: $2"
  Goto ffmpeg_done

ffmpeg_cleanup:
  IfFileExists "$2" 0 ffmpeg_done
  Delete "$2"

ffmpeg_done:
FunctionEnd
