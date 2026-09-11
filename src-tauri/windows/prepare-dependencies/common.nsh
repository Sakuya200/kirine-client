!include "LogicLib.nsh"
!include "StrFunc.nsh"

${StrRep}

!define POWERSHELL_EXE "$SYSDIR\WindowsPowerShell\v1.0\powershell.exe"

Function RunHiddenCommandWait
  Push $1
  Push $2

  nsExec::ExecToStack $0
  Pop $1
  Pop $2

  ${If} $1 == "error"
    StrCpy $0 "-1"
  ${Else}
    StrCpy $0 "$1"
  ${EndIf}

  Pop $2
  Pop $1
FunctionEnd

Function ExtractBundledZipArchive
  Exch $1
  Exch
  Exch $0

  ${StrRep} $0 "$0" "'" "''"
  ${StrRep} $1 "$1" "'" "''"
  StrCpy $2 "$\"${POWERSHELL_EXE}$\" -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -Command $\"Expand-Archive -LiteralPath '$0' -DestinationPath '$1' -Force$\""
  StrCpy $0 "$2"
  Call RunHiddenCommandWait
  StrCpy $2 "$0"

  Pop $0
  Push $2
FunctionEnd

Function MergeBundledZipArchiveContents
  Exch $1
  Exch
  Exch $0

  ${StrRep} $0 "$0" "'" "''"
  ${StrRep} $1 "$1" "'" "''"
  StrCpy $2 "$\"${POWERSHELL_EXE}$\" -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -Command $\"$$archive = '$0'; $$destination = '$1'; $$temp = Join-Path $$env:TEMP ('src-model-update-' + [System.IO.Path]::GetRandomFileName()); New-Item -ItemType Directory -Path $$temp -Force | Out-Null; Expand-Archive -LiteralPath $$archive -DestinationPath $$temp -Force; Get-ChildItem -LiteralPath $$temp -Recurse -File -Force | ForEach-Object { $$rel = $$_.FullName.Substring($$temp.Length).TrimStart('\'); $$target = Join-Path $$destination $$rel; $$targetDir = Split-Path $$target -Parent; if (-not (Test-Path -LiteralPath $$targetDir)) { New-Item -ItemType Directory -Path $$targetDir -Force | Out-Null }; Copy-Item -LiteralPath $$_.FullName -Destination $$target -Force }; Remove-Item -LiteralPath $$temp -Recurse -Force$\""
  StrCpy $0 "$2"
  Call RunHiddenCommandWait
  StrCpy $2 "$0"

  Pop $0
  Push $2
FunctionEnd

Function ClearInstallDirectory
  DetailPrint "Clearing leftover files from install directory: $INSTDIR"
  RMDir /r "$INSTDIR"
FunctionEnd

Function un.ClearInstallDirectory
  DetailPrint "Clearing leftover files from install directory: $INSTDIR"
  RMDir /r "$INSTDIR"
FunctionEnd