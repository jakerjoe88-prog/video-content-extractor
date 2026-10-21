@echo off
echo ========================================
echo    dy-mcp 抖音解析服务
echo ========================================
echo.

echo [1/2] 检查 Python 环境...
python --version
if errorlevel 1 (
    echo [错误] Python 未找到! 请安装 Python 3.10+ 并添加到 PATH.
    pause
    exit /b
)

echo.
echo [2/2] 安装依赖...
pip install -r requirements.txt -q
if errorlevel 1 (
    echo [错误] 依赖安装失败!
    pause
    exit /b
)

echo.
echo ========================================
echo    服务启动中...
echo    地址: http://127.0.0.1:38080
echo    按 Ctrl+C 停止服务
echo ========================================
echo.
python server.py
