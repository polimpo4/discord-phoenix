param(
    [switch]$NoStrip
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $root

$cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
if (-not (Test-Path $cargo)) {
    $cargo = "cargo"
}

Write-Host "Compilando Discord Phoenix (release)..." -ForegroundColor Cyan
& $cargo build --release
if ($LASTEXITCODE -ne 0) {
    throw "A compilacao falhou."
}

$exe = Join-Path $root "target\release\DiscordPhoenix.exe"
if (-not (Test-Path $exe)) {
    throw "Executavel nao encontrado em $exe"
}

New-Item -ItemType Directory -Force (Join-Path $root "dist") | Out-Null
Copy-Item $exe (Join-Path $root "dist\DiscordPhoenix.exe") -Force
Write-Host "Pronto: dist\DiscordPhoenix.exe" -ForegroundColor Green
