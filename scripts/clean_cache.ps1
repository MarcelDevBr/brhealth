# Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
# Licensed under the GNU Affero General Public License v3 (AGPLv3)
# or a commercial license agreement directly with the author.

[CmdletBinding()]
param(
    [switch]$Cargo,
    [switch]$All,
    [switch]$Force
)

$ErrorActionPreference = "Stop"
$CleanCargo = $Cargo -or $All

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Split-Path -Parent $ScriptDir
Set-Location $RootDir

Write-Host "================================================================" -ForegroundColor Cyan
Write-Host "     BRHealth - Limpeza de Caches e Temporários (Windows/PS)    " -ForegroundColor Cyan
Write-Host "================================================================" -ForegroundColor Cyan

# 1. Caches persistentes na pasta HOME do usuário (~/.brhealth)
Write-Host "`n[1/4] Removendo caches analíticos na pasta do usuário ($HOME\.brhealth)..." -ForegroundColor Yellow
$HomeDir = $HOME
$HomeTargets = @(
    (Join-Path $HomeDir ".brhealth\cache"),
    (Join-Path $HomeDir ".brhealth")
)

foreach ($Target in $HomeTargets) {
    if (Test-Path $Target) {
        Remove-Item -Recurse -Force $Target -ErrorAction SilentlyContinue
        Write-Host "✓ Removido: $Target" -ForegroundColor Green
    }
}

# Limpeza de eventuais resquícios legados no diretório temporário do sistema
$TempDir = [System.IO.Path]::GetTempPath()
$TempTargets = @(
    (Join-Path $TempDir "brhealth_cache"),
    (Join-Path $TempDir "brhealth_ffi_cache"),
    (Join-Path $TempDir "brhealth")
)

foreach ($Target in $TempTargets) {
    if (Test-Path $Target) {
        Remove-Item -Recurse -Force $Target -ErrorAction SilentlyContinue
        Write-Host "✓ Removido: $Target" -ForegroundColor Green
    }
}

# 2. Caches locais do repositório
Write-Host "`n[2/4] Removendo caches analíticos locais..." -ForegroundColor Yellow
$LocalTargets = @(
    (Join-Path $RootDir ".brhealth_cache"),
    (Join-Path $RootDir "data\cache"),
    (Join-Path $RootDir "target\hive_cache"),
    (Join-Path $RootDir "cache")
)

foreach ($Target in $LocalTargets) {
    if (Test-Path $Target) {
        Remove-Item -Recurse -Force $Target -ErrorAction SilentlyContinue
        Write-Host "✓ Removido: $Target" -ForegroundColor Green
    }
}

# 3. Caches de Python
Write-Host "`n[3/4] Removendo caches Python..." -ForegroundColor Yellow
Get-ChildItem -Path $RootDir -Recurse -Directory -Include "__pycache__",".pytest_cache",".ruff_cache",".mypy_cache" -ErrorAction SilentlyContinue | ForEach-Object {
    Remove-Item -Recurse -Force $_.FullName -ErrorAction SilentlyContinue
}
Get-ChildItem -Path $RootDir -Recurse -File -Include "*.pyc","*.pyo",".coverage" -ErrorAction SilentlyContinue | ForEach-Object {
    Remove-Item -Force $_.FullName -ErrorAction SilentlyContinue
}
Write-Host "✓ Caches Python limpos." -ForegroundColor Green

# 4. Cargo Clean opcional
if ($CleanCargo) {
    Write-Host "`n[4/4] Executando 'cargo clean'..." -ForegroundColor Yellow
    cargo clean
    Write-Host "✓ 'cargo clean' concluído com sucesso." -ForegroundColor Green
} else {
    Write-Host "`n[4/4] Artefatos do Cargo (target\) preservados." -ForegroundColor Yellow
}

Write-Host "`n================================================================" -ForegroundColor Cyan
Write-Host "        ✓ LIMPEZA DE CACHE CONCLUÍDA COM SUCESSO!               " -ForegroundColor Green
Write-Host "================================================================" -ForegroundColor Cyan
