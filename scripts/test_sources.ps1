# Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
# Licensed under the GNU Affero General Public License v3 (AGPLv3)
# or a commercial license agreement directly with the author.

[CmdletBinding()]
param(
    [switch]$List,
    [switch]$All,
    [string]$Pack,
    [string]$Source
)

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Split-Path -Parent $ScriptDir
Set-Location $RootDir

$Sources = @(
    # DATASUS
    @{ Fn="datasus_sim"; Id="datasus.sim"; Name="Mortalidade (SIM)"; Group="datasus" },
    @{ Fn="datasus_sinasc"; Id="datasus.sinasc"; Name="Nascidos Vivos (SINASC)"; Group="datasus" },
    @{ Fn="datasus_sih"; Id="datasus.sih"; Name="Internações Hospitalares (SIH/RD)"; Group="datasus" },
    @{ Fn="datasus_sinan"; Id="datasus.sinan"; Name="Agravos de Notificação (SINAN)"; Group="datasus" },
    @{ Fn="datasus_siasus"; Id="datasus.siasus"; Name="Ambulatorial (SIA-SUS)"; Group="datasus" },
    @{ Fn="datasus_cnes"; Id="datasus.cnes"; Name="Estabelecimentos de Saúde (CNES)"; Group="datasus" },
    @{ Fn="datasus_sipni"; Id="datasus.sipni"; Name="Imunizações (SI-PNI/RNDS)"; Group="datasus" },
    @{ Fn="datasus_sisvan"; Id="datasus.sisvan"; Name="Vigilância Alimentar e Nutricional (SISVAN)"; Group="datasus" },
    @{ Fn="datasus_siscan"; Id="datasus.siscan"; Name="Câncer / Mamografia / Colo (SISCAN)"; Group="datasus" },
    @{ Fn="datasus_bps"; Id="datasus.bps"; Name="Preços em Saúde e Fármacos (BPS/CMED)"; Group="datasus" },
    # IBGE
    @{ Fn="ibge_censo"; Id="ibge.censo"; Name="Censo Demográfico"; Group="ibge" },
    @{ Fn="ibge_pnad"; Id="ibge.pnad"; Name="PNAD Contínua"; Group="ibge" },
    @{ Fn="ibge_pof"; Id="ibge.pof"; Name="Pesquisa de Orçamentos Familiares (POF)"; Group="ibge" },
    @{ Fn="ibge_pense"; Id="ibge.pense"; Name="Pesquisa Nacional de Saúde do Escolar (PeNSE)"; Group="ibge" },
    @{ Fn="ibge_munic"; Id="ibge.munic"; Name="Pesquisa de Informações Básicas Municipais (MUNIC)"; Group="ibge" },
    # MDS
    @{ Fn="mds_cadunico"; Id="mds.cadunico"; Name="Cadastro Único para Programas Sociais (CadÚnico)"; Group="mds" },
    # AMBIENTAL
    @{ Fn="environmental_inmet"; Id="environmental.inmet"; Name="Estações Meteorológicas (INMET)"; Group="environmental" },
    @{ Fn="environmental_bdqueimadas"; Id="environmental.bdqueimadas"; Name="Focos de Queimadas (INPE/BDQueimadas)"; Group="environmental" },
    @{ Fn="environmental_prodes"; Id="environmental.prodes"; Name="Monitoramento de Desmatamento (INPE/PRODES)"; Group="environmental" },
    @{ Fn="environmental_sisagua"; Id="environmental.sisagua"; Name="Qualidade da Água para Consumo (SISAGUA)"; Group="environmental" },
    # GLOBAL
    @{ Fn="global_who_gho"; Id="global.who_gho"; Name="WHO Global Health Observatory"; Group="global" },
    @{ Fn="global_ihme_gbd"; Id="global.ihme_gbd"; Name="Global Burden of Disease (IHME GBD)"; Group="global" },
    @{ Fn="global_copernicus_era5"; Id="global.copernicus_era5"; Name="Copernicus ERA5-Land Reanalysis"; Group="global" },
    @{ Fn="global_worldpop"; Id="global.worldpop"; Name="WorldPop High Resolution Population Mapping"; Group="global" },
    @{ Fn="global_paho_plisa"; Id="global.paho_plisa"; Name="PAHO/OPAS Plataforma de Informação de Saúde"; Group="global" },
    @{ Fn="global_openaq"; Id="global.openaq"; Name="OpenAQ Global Air Quality Platform"; Group="global" }
)

if ($List) {
    Write-Host "=== Catálogo Oficial das 26 Fontes do BRHealth (PowerShell) ===" -ForegroundColor Cyan
    $Sources | Group-Object Group | ForEach-Object {
        Write-Host "`n▶ Grupo: $($_.Name.ToUpper()) ($($_.Count) fontes)" -ForegroundColor Magenta
        $_.Group | ForEach-Object {
            Write-Host ("  {0,-28} {1}" -f $_.Id, $_.Name)
        }
    }
    exit 0
}

$ToTest = @()

if ($All) {
    $ToTest = $Sources
} elseif ($Pack) {
    $P = $Pack.ToLower()
    if ($P -in @("brasil", "br", "national")) {
        $ToTest = $Sources | Where-Object { $_.Group -in @("datasus", "ibge", "mds", "environmental") }
    } elseif ($P -in @("global", "supranational")) {
        $ToTest = $Sources | Where-Object { $_.Group -eq "global" }
    } else {
        $ToTest = $Sources | Where-Object { $_.Group -eq $P }
    }
} elseif ($Source) {
    $Q = $Source.ToLower()
    $ToTest = $Sources | Where-Object { $_.Id.ToLower().Contains($Q) -or $_.Fn.ToLower().Contains($Q) }
} else {
    Write-Host "Uso: .\scripts\test_sources.ps1 [-All] [-List] [-Pack <nome>] [-Source <nome>]" -ForegroundColor Yellow
    exit 0
}

Write-Host "================================================================" -ForegroundColor Cyan
Write-Host "   BRHealth - Teste de Fontes Analíticas ($($ToTest.Count) selecionadas)   " -ForegroundColor Cyan
Write-Host "================================================================`n" -ForegroundColor Cyan

$Failed = 0
foreach ($Item in $ToTest) {
    Write-Host ("  {0,-28} " -f $Item.Id) -NoNewline
    $Stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    
    cargo test -q --test test_each_source "test_source_$($Item.Fn)" -- --exact 2>&1 | Out-Null
    $Success = ($LASTEXITCODE -eq 0)
    $Stopwatch.Stop()
    
    if ($Success) {
        Write-Host "✓ APROVADO ($($Stopwatch.ElapsedMilliseconds)ms) - $($Item.Name)" -ForegroundColor Green
    } else {
        Write-Host "✗ FALHOU - $($Item.Name)" -ForegroundColor Red
        $Failed++
    }
}

Write-Host "`n================================================================" -ForegroundColor Cyan
if ($Failed -eq 0) {
    Write-Host "        ✓ TODAS AS $($ToTest.Count) FONTES FORAM APROVADAS!     " -ForegroundColor Green
    Write-Host "================================================================" -ForegroundColor Cyan
    exit 0
} else {
    Write-Host "        ✗ FALHA NO TESTE DE $Failed FONTE(S)!                   " -ForegroundColor Red
    Write-Host "================================================================" -ForegroundColor Cyan
    exit 1
}
