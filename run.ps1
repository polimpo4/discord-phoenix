$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe = Join-Path $root "target\release\DiscordPhoenix.exe"

if (-not (Test-Path $exe)) {
    Write-Host "Executavel nao encontrado. Compilando primeiro..." -ForegroundColor Yellow
    & (Join-Path $root "build.ps1")
    $exe = Join-Path $root "dist\DiscordPhoenix.exe"
}

Start-Process -FilePath $exe -WorkingDirectory $root
