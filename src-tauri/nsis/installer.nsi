; 自定义 NSIS 安装脚本 - Tauri 2 格式
; 用于卸载时清理用户数据

!include "LogicLib.nsh"

; ============================================
; 安装后初始化 (Tauri 2.0 格式)
; ============================================
!macro NSIS_HOOK_POSTINSTALL
    ; 创建数据目录结构
    CreateDirectory "$APPDATA\DouyinCreatorToolkit"
    CreateDirectory "$APPDATA\DouyinCreatorToolkit\config"
    CreateDirectory "$APPDATA\DouyinCreatorToolkit\logs"
    CreateDirectory "$APPDATA\DouyinCreatorToolkit\models"
    CreateDirectory "$APPDATA\DouyinCreatorToolkit\models\asr"
    CreateDirectory "$APPDATA\DouyinCreatorToolkit\models\bge-small-zh"
    CreateDirectory "$APPDATA\DouyinCreatorToolkit\data"
    CreateDirectory "$APPDATA\DouyinCreatorToolkit\temp"
    ; DLL 文件已由 Tauri 自动打包到 exe 同级目录，无需手动复制
!macroend

; ============================================
; 卸载前清理 (Tauri 2.0 格式)
; ============================================
!macro NSIS_HOOK_PREUNINSTALL
    ; 强制删除安装目录下的 resources 文件夹（包含 python-embed 等）
    ; 这些目录结构复杂，NSIS 默认卸载可能删不干净
    RMDir /r "$INSTDIR\resources"
    
    ; 询问用户是否删除数据
    MessageBox MB_YESNO|MB_ICONQUESTION "是否删除所有用户数据？$\r$\n$\r$\n将删除以下内容：$\r$\n• 语音识别模型 (约 200MB)$\r$\n• 知识库嵌入模型 (约 90MB)$\r$\n• 知识库数据和数据库$\r$\n• 配置文件和日志$\r$\n• 临时文件$\r$\n$\r$\n存储位置: $APPDATA\DouyinCreatorToolkit$\r$\n$\r$\n选择「否」可在重装后继续使用已下载的模型。" IDYES deleteData IDNO skipDelete
    
    deleteData:
        ; 删除 Roaming AppData 数据目录（包含模型、配置、日志等）
        RMDir /r "$APPDATA\DouyinCreatorToolkit"
        
        ; 删除 Local AppData 数据目录（可能包含缓存）
        RMDir /r "$LOCALAPPDATA\DouyinCreatorToolkit"
        
        ; 删除 Tauri 默认的应用数据目录
        RMDir /r "$LOCALAPPDATA\com.douyin.creator-toolkit"
        
        ; 删除注册表残留
        DeleteRegKey HKCU "Software\DouyinCreatorToolkit"
        DeleteRegKey HKCU "Software\com.douyin.creator-toolkit"
        
        Goto done
    
    skipDelete:
        ; 用户选择不删除，什么都不做
    
    done:
!macroend
