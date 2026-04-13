Function InstallBundledSrcModelRuntime
  StrCpy $0 "$INSTDIR\lib\src-model"
  StrCpy $1 "$0\scripts\windows\init_task_runtime.ps1"
  StrCpy $4 "$0\scripts\windows\download_models.ps1"
  StrCpy $2 "$INSTDIR\resources\src-model-runtime.zip"

  DetailPrint "Checking bundled src-model runtime..."
  IfFileExists "$2" 0 src_model_missing_archive

  CreateDirectory "$0"
  IfFileExists "$1" 0 src_model_extract_start
  IfFileExists "$4" src_model_extract_update src_model_extract_start

src_model_extract_update:
  DetailPrint "Updating bundled src-model runtime in $0"
  Goto src_model_extract_archive

src_model_extract_start:
  DetailPrint "Extracting bundled src-model runtime to $0"

src_model_extract_archive:
  Push "$2"
  Push "$0"
  Call ExtractBundledZipArchiveContents
  Pop $3
  ${If} $3 != 0
    MessageBox MB_ICONEXCLAMATION "src-model runtime extraction failed with exit code $3. Model tasks may not work correctly."
    Goto src_model_cleanup
  ${EndIf}

  IfFileExists "$1" 0 src_model_missing_scripts
  IfFileExists "$4" src_model_cleanup src_model_missing_scripts

src_model_missing_scripts:
  MessageBox MB_ICONEXCLAMATION "src-model runtime extraction completed, but init_task_runtime.ps1 or download_models.ps1 was not found in $0\scripts\windows."
  Goto src_model_cleanup

src_model_missing_archive:
  MessageBox MB_ICONEXCLAMATION "Bundled src-model runtime archive was not found: $2"

src_model_cleanup:
  IfFileExists "$2" 0 src_model_done
  Delete "$2"

src_model_done:
FunctionEnd