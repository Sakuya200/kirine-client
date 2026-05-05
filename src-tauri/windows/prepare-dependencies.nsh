!define PREPARE_DEPENDENCIES_DIR "${__FILEDIR__}\prepare-dependencies"

!include "${PREPARE_DEPENDENCIES_DIR}\common.nsh"
!include "${PREPARE_DEPENDENCIES_DIR}\sox.nsh"
!include "${PREPARE_DEPENDENCIES_DIR}\ffmpeg.nsh"
!include "${PREPARE_DEPENDENCIES_DIR}\src-model.nsh"

Function RelocateBundledAppFile
	Exch $1
	Exch
	Exch $0
	Push $2

	IfFileExists "$0" 0 relocate_done
	IfFileExists "$1" app_file_exists move_app_file

move_app_file:
	DetailPrint "Moving bundled file to $1"
	ClearErrors
	Rename "$0" "$1"
	IfErrors move_app_file_failed relocate_done

app_file_exists:
	DetailPrint "$1 already exists, keeping the existing file."
	Delete "$0"
	Goto relocate_done

move_app_file_failed:
	MessageBox MB_ICONEXCLAMATION "Failed to move bundled file to $1. The bundled resources directory will still be cleaned up."

relocate_done:
	Pop $2
	Pop $0
	Pop $1
FunctionEnd

Function RelocateBundledAppFilesAndCleanupResources
	StrCpy $0 "$INSTDIR\resources"

	Push "$INSTDIR\resources\config.toml"
	Push "$INSTDIR\config.toml"
	Call RelocateBundledAppFile

	Push "$INSTDIR\resources\supported_models.json"
	Push "$INSTDIR\supported_models.json"
	Call RelocateBundledAppFile

	IfFileExists "$0" 0 cleanup_done
	DetailPrint "Removing bundled resources directory: $0"
	RMDir /r "$0"

cleanup_done:
FunctionEnd

!macro NSIS_HOOK_POSTINSTALL
	CreateDirectory "$INSTDIR\lib"
	Call InstallBundledSox
	Call InstallBundledFfmpeg
	Call InstallBundledSrcModelRuntime
	Call RelocateBundledAppFilesAndCleanupResources
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
	${If} $DeleteAppDataCheckboxState = 1
	${AndIf} $UpdateMode <> 1
		Call un.ClearInstallDirectory
	${EndIf}
!macroend
