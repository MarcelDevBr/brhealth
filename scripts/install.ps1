# Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
# Licensed under the GNU Affero General Public License v3 (AGPLv3)
# or a commercial license agreement directly with the author.

[CmdletBinding()]
param (
    [string]$Version = $env:BRHEALTH_VERSION,
    [string]$InstallDir = ""
)

$ErrorActionPreference = "Stop"

$Repo = "MarcelDevBr/brhealth"
$Target = "x86_64-pc-windows-msvc"

Write-Host "========================================================" -ForegroundColor Cyan
Write-Host "    BRHealth CLI - Instalador Windows (PowerShell)     " -ForegroundColor Cyan
Write-Host "========================================================" -ForegroundColor Cyan

# 1. Determinar versão
if (-not $Version) {
    Write-Host "-> Buscando a versão estável mais recente..." -ForegroundColor Yellow
    try {
        $ReleaseUrl = "https://api.github.com/repos/$Repo/releases/latest"
        $Response = Invoke-RestMethod -Uri $ReleaseUrl -Headers @{ "Accept" = "application/vnd.github.v3+json" } -UseBasicParsing
        $Version = $Response.tag_name
        Write-Host "-> Versão mais recente encontrada: $Version" -ForegroundColor Green
    }
    catch {
        $Version = "v0.1.0"
        Write-Host "-> Não foi possível consultar API do GitHub. Usando versão padrão: $Version" -ForegroundColor Yellow
    }
} else {
    Write-Host "-> Usando versão especificada: $Version" -ForegroundColor Green
}

# 2. URLs de Download
$ArchiveName = "brhealth-$Version-$Target.zip"
$DownloadUrl = "https://github.com/$Repo/releases/download/$Version/$ArchiveName"
$ChecksumUrl = "$DownloadUrl.sha256"

$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null

try {
    $ZipPath = Join-Path $TempDir $ArchiveName
    Write-Host "-> Baixando $ArchiveName..." -ForegroundColor Yellow
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath -UseBasicParsing

    # 3. Verificação de integridade SHA-256
    $ShaFile = Join-Path $TempDir "$ArchiveName.sha256"
    try {
        Invoke-WebRequest -Uri $ChecksumUrl -OutFile $ShaFile -UseBasicParsing
        $ExpectedHash = (Get-Content -Path $ShaFile).Trim().Split(" ")[0].ToLower()
        $ActualHash = (Get-FileHash -Path $ZipPath -Algorithm SHA256).Hash.ToLower()

        if ($ExpectedHash -eq $ActualHash) {
            Write-Host "-> Integridade SHA-256 verificada com sucesso!" -ForegroundColor Green
        } else {
            Write-Error "Falha de integridade: SHA-256 esperado ($ExpectedHash) não confere com o obtido ($ActualHash)."
            exit 1
        }
    } catch {
        Write-Host "-> Arquivo de checksum não encontrado ou validação ignorada." -ForegroundColor Yellow
    }

    # 4. Descompactar
    Write-Host "-> Extraindo arquivo zip..." -ForegroundColor Yellow
    Expand-Archive -Path $ZipPath -DestinationPath $TempDir -Force

    $SourceExe = Join-Path $TempDir "brhealth.exe"
    if (-not (Test-Path $SourceExe)) {
        Write-Error "Binário 'brhealth.exe' não encontrado dentro do pacote descompactado."
        exit 1
    }

    # 5. Diretório de Destino
    if (-not $InstallDir) {
        $InstallDir = Join-Path $env:LOCALAPPDATA "Programs\brhealth\bin"
    }

    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }

    $TargetExe = Join-Path $InstallDir "brhealth.exe"
    Copy-Item -Path $SourceExe -Destination $TargetExe -Force
    Write-Host "-> Instalado em: $TargetExe" -ForegroundColor Green

    # 6. Atualizar PATH do Usuário se necessário
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($UserPath -notlike "*$InstallDir*") {
        Write-Host "-> Adicionando $InstallDir ao PATH do Usuário..." -ForegroundColor Yellow
        $NewPath = "$UserPath;$InstallDir"
        [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
        $env:Path = "$env:Path;$InstallDir"
        Write-Host "-> PATH atualizado com sucesso!" -ForegroundColor Green
    }

    Write-Host ""
    Write-Host "========================================================" -ForegroundColor Cyan
    Write-Host "    BRHealth CLI instalado com sucesso no Windows!     " -ForegroundColor Cyan
    Write-Host "========================================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Execute para testar:"
    Write-Host "    brhealth --help"
    Write-Host "    brhealth dv 355030"
    Write-Host ""
}
finally {
    Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
}
