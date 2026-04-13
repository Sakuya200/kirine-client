!include "LogicLib.nsh"
!include "StrFunc.nsh"
!include "WinMessages.nsh"

${StrStr}
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

Function BroadcastEnvironmentChange
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
FunctionEnd

Function AddToUserPathIfMissing
  Exch $0
  Push $1
  Push $2
  Push $3

  ReadRegStr $1 HKCU "Environment" "Path"

  ${If} $1 == ""
    WriteRegExpandStr HKCU "Environment" "Path" "$0"
    Call BroadcastEnvironmentChange
    Goto done
  ${EndIf}

  StrCpy $2 ";$1;"
  StrCpy $3 ";$0;"
  ${StrStr} $2 $2 $3

  ${If} $2 == ""
    WriteRegExpandStr HKCU "Environment" "Path" "$1;$0"
    Call BroadcastEnvironmentChange
  ${EndIf}

done:
  Pop $3
  Pop $2
  Pop $1
  Pop $0
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

Function ExtractBundledZipArchiveContents
  Exch $1
  Exch
  Exch $0

  ${StrRep} $0 "$0" "'" "''"
  ${StrRep} $1 "$1" "'" "''"
  StrCpy $2 "$\"${POWERSHELL_EXE}$\" -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -Command $\"$tempDir = Join-Path $env:TEMP ('kirine-src-model-' + [guid]::NewGuid().ToString()); New-Item -ItemType Directory -Path $tempDir -Force | Out-Null; try { Expand-Archive -LiteralPath '$0' -DestinationPath $tempDir -Force; Get-ChildItem -LiteralPath $tempDir | ForEach-Object { Copy-Item -LiteralPath $_.FullName -Destination '$1' -Recurse -Force } } finally { if (Test-Path -LiteralPath $tempDir) { Remove-Item -LiteralPath $tempDir -Recurse -Force } }$\""
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