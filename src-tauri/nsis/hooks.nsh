; 自定义 NSIS 安装钩子 - Tauri 2 格式
; 用于安装后初始化和卸载时清理

!include "LogicLib.nsh"

; ============================================
; 安装后初始化
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
; 卸载前清理
; ============================================
!macro NSIS_HOOK_PREUNINSTALL
    ; 关键修复：先终止所有 Python 进程，否则目录可能无法删除
    ; 使用 nsExec 静默执行 taskkill
    nsExec::ExecToLog 'taskkill /F /IM python.exe /T'
    nsExec::ExecToLog 'taskkill /F /IM pythonw.exe /T'
    
    ; 等待进程完全退出
    Sleep 1000
    
    ; 强制删除 Python 环境目录（这些目录包含大量文件，是残留的主要原因）
    RMDir /r "$INSTDIR\resources\python-env"
    RMDir /r "$INSTDIR\resources\python-embed"
    RMDir /r "$INSTDIR\resources\dy-mcp"
    RMDir /r "$INSTDIR\resources\ffmpeg"
    
    ; 删除整个 resources 目录（兜底）
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
