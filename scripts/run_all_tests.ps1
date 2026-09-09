# Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
# Licensed under the GNU Affero General Public License v3 (AGPLv3)
# or a commercial license agreement directly with the author.

[CmdletBinding()]
param()

$ErrorActionPreference = "Continue"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Split-Path -Parent $ScriptDir
Set-Location $RootDir

Write-Host "================================================================" -ForegroundColor Cyan
Write-Host "    BRHealth - Execução Completa de Testes (Windows/PowerShell) " -ForegroundColor Cyan
Write-Host "================================================================" -ForegroundColor Cyan
Write-Host "Raiz do projeto: $RootDir"
Write-Host "Data / Hora:     $([DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ'))`n"

$Failed = 0
$TotalTimer = [System.Diagnostics.Stopwatch]::StartNew()

function Run-Step($Title, [scriptblock]$Command) {
    Write-Host "`n[ETAPA] $Title" -ForegroundColor Yellow
    $StepTimer = [System.Diagnostics.Stopwatch]::StartNew()
    & $Command
    $StepTimer.Stop()

    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ $Title concluído com sucesso ($($StepTimer.Elapsed.TotalSeconds.ToString('F2'))s)" -ForegroundColor Green
    } else {
        Write-Host "✗ $Title FALHOU ($($StepTimer.Elapsed.TotalSeconds.ToString('F2'))s)" -ForegroundColor Red
        $script:Failed++
    }
}

Run-Step "Verificação Estática de Tipos (cargo check)" { cargo check --workspace --all-targets }
Run-Step "Linter de Alta Rigidez (cargo clippy)" { cargo clippy --workspace --all-targets -- -D warnings }
Run-Step "Testes Unitários e de Integração" { cargo test -p brhealth-core -p brhealth-ffi -p brhealth-jni -p brhealth-cli }
Run-Step "Testes do Módulo Python" { cargo test -p brhealth-python }
Run-Step "Doc-tests da Documentação Formal (LaTeX)" { cargo test --doc }
Run-Step "Validação Granular das 26 Fontes Oficiais" { cargo test --test test_each_source }
Run-Step "Testes da Interface CLI Nativa" { cargo test --test test_cli }

$TotalTimer.Stop()

Write-Host "`n================================================================" -ForegroundColor Cyan
if ($Failed -eq 0) {
    Write-Host "        ✓ TODOS OS TESTES FORAM APROVADOS COM SUCESSO!         " -ForegroundColor Green
    Write-Host "        Tempo Total: $($TotalTimer.Elapsed.TotalSeconds.ToString('F2'))s" -ForegroundColor Green
    Write-Host "================================================================" -ForegroundColor Cyan
    exit 0
} else {
    Write-Host "        ✗ $Failed ETAPA(S) DE TESTE FALHARAM!                   " -ForegroundColor Red
    Write-Host "        Tempo Total: $($TotalTimer.Elapsed.TotalSeconds.ToString('F2'))s" -ForegroundColor Red
    Write-Host "================================================================" -ForegroundColor Cyan
    exit 1
}
