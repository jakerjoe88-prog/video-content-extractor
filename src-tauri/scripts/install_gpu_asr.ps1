# GPU ASR 安装脚本
# 安装 sherpa-onnx Python 绑定以启用 GPU 加速

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  GPU 语音识别加速安装脚本" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 检查 Python
$pythonCmd = $null
foreach ($cmd in @("python", "python3", "py")) {
    try {
        $version = & $cmd --version 2>&1
        if ($version -match "Python 3") {
            $pythonCmd = $cmd
            Write-Host "[OK] 找到 Python: $version" -ForegroundColor Green
            break
        }
    } catch {}
}

if (-not $pythonCmd) {
    Write-Host "[错误] 未找到 Python 3，请先安装 Python 3.8+" -ForegroundColor Red
    Write-Host "下载地址: https://www.python.org/downloads/" -ForegroundColor Yellow
    exit 1
}

# 检查 pip
Write-Host ""
Write-Host "正在检查 pip..." -ForegroundColor Cyan
& $pythonCmd -m pip --version
if ($LASTEXITCODE -ne 0) {
    Write-Host "[错误] pip 不可用" -ForegroundColor Red
    exit 1
}

# 安装依赖
Write-Host ""
Write-Host "正在安装 GPU ASR 依赖..." -ForegroundColor Cyan
Write-Host "这可能需要几分钟时间..." -ForegroundColor Yellow
Write-Host ""

# 安装 sherpa-onnx
& $pythonCmd -m pip install sherpa-onnx fastapi uvicorn --upgrade

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Green
    Write-Host "  安装成功！" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
    Write-Host ""
    Write-Host "GPU 加速已准备就绪。" -ForegroundColor Cyan
    Write-Host "重启应用后，在设置中启用 GPU 加速即可。" -ForegroundColor Cyan
} else {
    Write-Host ""
    Write-Host "[错误] 安装失败" -ForegroundColor Red
    Write-Host "请检查网络连接或手动运行:" -ForegroundColor Yellow
    Write-Host "  pip install sherpa-onnx fastapi uvicorn" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "按任意键退出..."
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
