!define PREPARE_DEPENDENCIES_DIR "${__FILEDIR__}\prepare-dependencies"

!include "${PREPARE_DEPENDENCIES_DIR}\common.nsh"
!include "${PREPARE_DEPENDENCIES_DIR}\sox.nsh"
!include "${PREPARE_DEPENDENCIES_DIR}\ffmpeg.nsh"
!include "${PREPARE_DEPENDENCIES_DIR}\src-model.nsh"

Function RelocateBundledConfigAndCleanupResources
	StrCpy $0 "$INSTDIR\resources\config.toml"
	StrCpy $1 "$INSTDIR\config.toml"
	StrCpy $2 "$INSTDIR\resources"

	IfFileExists "$0" 0 cleanup_resources
	IfFileExists "$1" config_exists move_config

move_config:
	DetailPrint "Moving bundled config.toml to $1"
	ClearErrors
	Rename "$0" "$1"
	IfErrors move_failed cleanup_resources

config_exists:
	DetailPrint "config.toml already exists at $1, keeping the existing file."
	Delete "$0"
	Goto cleanup_resources

move_failed:
	MessageBox MB_ICONEXCLAMATION "Failed to move bundled config.toml to $1. The bundled resources directory will still be cleaned up."

cleanup_resources:
	IfFileExists "$2" 0 cleanup_done
	DetailPrint "Removing bundled resources directory: $2"
	RMDir /r "$2"

cleanup_done:
FunctionEnd

!macro NSIS_HOOK_POSTINSTALL
	CreateDirectory "$INSTDIR\lib"
	Call InstallBundledSox
	Call InstallBundledFfmpeg
	Call InstallBundledSrcModelRuntime
	Call RelocateBundledConfigAndCleanupResources
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
	${If} $DeleteAppDataCheckboxState = 1
	${AndIf} $UpdateMode <> 1
		Call un.ClearInstallDirectory
	${EndIf}
!macroend
