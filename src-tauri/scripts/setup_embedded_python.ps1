param(
    [switch]$SkipPythonPackages
)

$ErrorActionPreference = 'Stop'

& (Join-Path $PSScriptRoot 'prepare_runtime.ps1') -PythonOnly -SkipPythonPackages:$SkipPythonPackages
