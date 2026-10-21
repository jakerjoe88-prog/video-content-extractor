param(
    [switch]$PythonOnly,
    [switch]$FfmpegOnly,
    [switch]$SkipPythonPackages,
    [switch]$ManifestOnly
)

$ErrorActionPreference = 'Stop'

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$srcTauriDir = Split-Path -Parent $scriptRoot
$resourcesDir = Join-Path $srcTauriDir 'resources'
$pythonTargetDir = Join-Path $resourcesDir 'python-embed'
$ffmpegTargetDir = Join-Path $resourcesDir 'ffmpeg'
$binTargetDir = Join-Path $resourcesDir 'bin'
$manifestPath = Join-Path $resourcesDir 'runtime-manifest.json'
$requirementsPath = Join-Path $resourcesDir 'dy-mcp/requirements.txt'

$pythonVersion = '3.10.11'
$pythonUrl = "https://www.python.org/ftp/python/$pythonVersion/python-$pythonVersion-embed-amd64.zip"
$ffmpegUrl = 'https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip'
$dllFiles = @(
    'DirectML.dll',
    'onnxruntime.dll',
    'sherpa-onnx-c-api.dll',
    'sherpa-onnx-cxx-api.dll'
)

function Write-Step([string]$message) {
    Write-Host "`n==> $message" -ForegroundColor Cyan
}

function Ensure-Directory([string]$path) {
    if (-not (Test-Path -LiteralPath $path)) {
        New-Item -ItemType Directory -Path $path -Force | Out-Null
    }
}

function New-StagingDirectory([string]$name) {
    $path = Join-Path ([System.IO.Path]::GetTempPath()) ("douyin-runtime-$name-" + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $path -Force | Out-Null
    return $path
}

function Download-File([string]$url, [string]$outputPath) {
    Write-Host "Downloading: $url"
    Invoke-WebRequest -Uri $url -OutFile $outputPath
}

function Copy-StagingContent([string]$sourceDir, [string]$targetDir) {
    Ensure-Directory $targetDir
    $items = Get-ChildItem -LiteralPath $sourceDir -Force
    foreach ($item in $items) {
        Copy-Item -LiteralPath $item.FullName -Destination $targetDir -Recurse -Force
    }
}

function Update-PythonPth([string]$pythonDir) {
    $pthFile = Get-ChildItem -LiteralPath $pythonDir -Filter 'python*._pth' | Select-Object -First 1
    if (-not $pthFile) {
        throw 'Missing python*._pth. Unable to configure embedded Python.'
    }

    $lines = [System.Collections.Generic.List[string]]::new()
    foreach ($line in Get-Content -LiteralPath $pthFile.FullName) {
        if ($line -eq '#import site') {
            $lines.Add('import site')
        } elseif ($line -ne 'Lib' -and $line -ne 'Lib/site-packages') {
            $lines.Add($line)
        }
    }

    if (-not ($lines -contains 'Lib')) {
        $lines.Add('Lib')
    }
    if (-not ($lines -contains 'Lib/site-packages')) {
        $lines.Add('Lib/site-packages')
    }
    if (-not ($lines -contains 'import site')) {
        $lines.Add('import site')
    }

    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllLines($pthFile.FullName, $lines, $utf8NoBom)
}

function Test-EmbeddedPythonPackages([string]$pythonDir) {
    $pythonExe = Join-Path $pythonDir 'python.exe'
    & $pythonExe -c "import requests, fastapi, uvicorn, pydantic, sherpa_onnx"
}

function Install-EmbeddedPythonPackages([string]$pythonDir) {
    if ($SkipPythonPackages) {
        Write-Host 'Skip Python package installation.' -ForegroundColor Yellow
        return
    }

    if (-not (Test-Path -LiteralPath $requirementsPath)) {
        throw "Missing requirements file: $requirementsPath"
    }

    $pythonExe = Join-Path $pythonDir 'python.exe'
    $sitePackages = Join-Path $pythonDir 'Lib/site-packages'
    $getPipPath = Join-Path $pythonDir 'get-pip.py'

    Ensure-Directory (Join-Path $pythonDir 'Lib')
    Ensure-Directory $sitePackages

    Write-Step 'Install pip into embedded Python'
    Download-File 'https://bootstrap.pypa.io/get-pip.py' $getPipPath
    & $pythonExe $getPipPath --no-warn-script-location

    Write-Step 'Install Python dependencies for sidecar'
    & $pythonExe -m pip install --disable-pip-version-check --no-warn-script-location --no-cache-dir --upgrade --target $sitePackages -r $requirementsPath

    Write-Step 'Verify embedded Python imports'
    Test-EmbeddedPythonPackages $pythonDir

    if (Test-Path -LiteralPath $getPipPath) {
        Remove-Item -LiteralPath $getPipPath -Force
    }
}

function Prepare-EmbeddedPython {
    Write-Step 'Prepare python-embed runtime'
    Ensure-Directory $pythonTargetDir

    $downloadZip = Join-Path (New-StagingDirectory 'download-python') 'python-embed.zip'
    $extractDir = New-StagingDirectory 'extract-python'

    Download-File $pythonUrl $downloadZip
    Expand-Archive -LiteralPath $downloadZip -DestinationPath $extractDir -Force
    Copy-StagingContent $extractDir $pythonTargetDir
    Update-PythonPth $pythonTargetDir
    Install-EmbeddedPythonPackages $pythonTargetDir
}

function Prepare-Ffmpeg {
    Write-Step 'Prepare FFmpeg runtime'
    Ensure-Directory $ffmpegTargetDir

    $downloadZip = Join-Path (New-StagingDirectory 'download-ffmpeg') 'ffmpeg.zip'
    $extractDir = New-StagingDirectory 'extract-ffmpeg'

    Download-File $ffmpegUrl $downloadZip
    Expand-Archive -LiteralPath $downloadZip -DestinationPath $extractDir -Force

    $ffmpegExe = Get-ChildItem -LiteralPath $extractDir -Filter 'ffmpeg.exe' -Recurse | Select-Object -First 1
    $ffprobeExe = Get-ChildItem -LiteralPath $extractDir -Filter 'ffprobe.exe' -Recurse | Select-Object -First 1

    if (-not $ffmpegExe -or -not $ffprobeExe) {
        throw 'FFmpeg archive does not contain ffmpeg.exe or ffprobe.exe.'
    }

    Copy-Item -LiteralPath $ffmpegExe.FullName -Destination (Join-Path $ffmpegTargetDir 'ffmpeg.exe') -Force
    Copy-Item -LiteralPath $ffprobeExe.FullName -Destination (Join-Path $ffmpegTargetDir 'ffprobe.exe') -Force
}

function Sync-BundledDlls {
    Write-Step 'Sync runtime DLLs into resources/bin'
    Ensure-Directory $binTargetDir

    foreach ($dll in $dllFiles) {
        $sourcePath = Join-Path $srcTauriDir $dll
        $targetPath = Join-Path $binTargetDir $dll

        if (-not (Test-Path -LiteralPath $sourcePath) -and -not (Test-Path -LiteralPath $targetPath)) {
            throw "Missing DLL resource: $dll"
        }

        if (Test-Path -LiteralPath $sourcePath) {
            Copy-Item -LiteralPath $sourcePath -Destination $targetPath -Force
        }
    }
}

function Get-CommandOutputLine([string]$exePath, [string[]]$arguments) {
    if (-not (Test-Path -LiteralPath $exePath)) {
        return $null
    }

    $escapedArgs = @(
        foreach ($argument in $arguments) {
            if ($argument -match '\s') {
                '"{0}"' -f $argument.Replace('"', '\"')
            } else {
                $argument
            }
        }
    )

    $commandLine = '"{0}" {1} 2>&1' -f $exePath, ($escapedArgs -join ' ')
    $output = cmd /d /c $commandLine
    return ($output | Where-Object { $_ } | Select-Object -First 1)
}

function Write-RuntimeManifest {
    Write-Step 'Write runtime manifest'

    $pythonExe = Join-Path $pythonTargetDir 'python.exe'
    $ffmpegExe = Join-Path $ffmpegTargetDir 'ffmpeg.exe'
    $ffprobeExe = Join-Path $ffmpegTargetDir 'ffprobe.exe'

    $manifest = [ordered]@{
        generatedAt = (Get-Date).ToString('o')
        python = [ordered]@{
            version = $pythonVersion
            source = $pythonUrl
            executable = $pythonExe
            resolvedVersion = Get-CommandOutputLine $pythonExe @('--version')
        }
        ffmpeg = [ordered]@{
            source = $ffmpegUrl
            ffmpeg = [ordered]@{
                path = $ffmpegExe
                version = Get-CommandOutputLine $ffmpegExe @('-version')
            }
            ffprobe = [ordered]@{
                path = $ffprobeExe
                version = Get-CommandOutputLine $ffprobeExe @('-version')
            }
        }
        dlls = @(
            foreach ($dll in $dllFiles) {
                $path = Join-Path $binTargetDir $dll
                [ordered]@{
                    name = $dll
                    path = $path
                    exists = Test-Path -LiteralPath $path
                }
            }
        )
        pythonRequirements = if (Test-Path -LiteralPath $requirementsPath) {
            @((Get-Content -LiteralPath $requirementsPath) | ForEach-Object { [string]$_ } | Where-Object { $_ -and -not $_.StartsWith('#') })
        } else {
            @()
        }
    }

    $manifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $manifestPath -Encoding UTF8
}

Write-Host 'Preparing distributable runtime resources...' -ForegroundColor Green
Write-Host "src-tauri: $srcTauriDir"

if (-not $ManifestOnly) {
    if (-not $FfmpegOnly) {
        Prepare-EmbeddedPython
    }

    if (-not $PythonOnly) {
        Prepare-Ffmpeg
    }

    Sync-BundledDlls
}

Write-RuntimeManifest

Write-Host "`nDone." -ForegroundColor Green
Write-Host "python-embed: $pythonTargetDir"
Write-Host "ffmpeg: $ffmpegTargetDir"
Write-Host "manifest: $manifestPath"
