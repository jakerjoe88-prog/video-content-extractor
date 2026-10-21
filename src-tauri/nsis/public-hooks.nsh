; Public installer: data and browser profiles are created by the app on first run.
; Tauri removes only the files recorded by this installer on uninstall.
; Never terminate unrelated Python processes or touch development/Douyin data.
!macro NSIS_HOOK_POSTINSTALL
!macroend
!macro NSIS_HOOK_PREUNINSTALL
!macroend
