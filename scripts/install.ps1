<#
.SYNOPSIS
Installs localreel-plugin-cli for Windows x64.

.EXAMPLE
irm https://raw.githubusercontent.com/LocalReel/localreel-plugin-cli/main/scripts/install.ps1 | iex

.EXAMPLE
.\install.ps1 -Version v0.2.0 -AddToPath
#>
[CmdletBinding()]
param(
    [string]$Version = 'latest',
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA 'localreel-plugin-cli'),
    [switch]$AddToPath
)

$ErrorActionPreference = 'Stop'
$archive = 'localreel-plugin-cli-windows-x64.zip'
if ($Version -eq 'latest') {
    $base = 'https://github.com/LocalReel/localreel-plugin-cli/releases/latest/download'
} else {
    $base = "https://github.com/LocalReel/localreel-plugin-cli/releases/download/$Version"
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("localreel-plugin-cli-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
    Write-Host "downloading $archive ..."
    $zip = Join-Path $tmp $archive
    Invoke-WebRequest -Uri "$base/$archive" -OutFile $zip -UseBasicParsing
    $checksumFile = Join-Path $tmp "$archive.sha256"
    try {
        Invoke-WebRequest -Uri "$base/$archive.sha256" -OutFile $checksumFile -UseBasicParsing
        $expected = (Get-Content $checksumFile -Raw).Split(' ')[0].Trim().ToLowerInvariant()
        $actual = (Get-FileHash -Path $zip -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($expected -ne $actual) { throw "checksum mismatch: expected $expected, got $actual" }
        Write-Host 'checksum ok'
    } catch [System.Net.WebException] {
        Write-Warning 'checksum file unavailable; skipping verification'
    }

    $extract = Join-Path $tmp 'extract'
    Expand-Archive -Path $zip -DestinationPath $extract -Force
    $binary = Get-ChildItem -Path $extract -Recurse -Filter 'localreel-plugin-cli.exe' |
        Select-Object -First 1
    if (-not $binary) { throw 'archive did not contain localreel-plugin-cli.exe' }

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item $binary.FullName (Join-Path $InstallDir 'localreel-plugin-cli.exe') -Force
    Write-Host "installed $(Join-Path $InstallDir 'localreel-plugin-cli.exe')"

    if ($AddToPath) {
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        if (-not (($userPath -split ';') -contains $InstallDir)) {
            [Environment]::SetEnvironmentVariable('Path', "$userPath;$InstallDir", 'User')
            Write-Host "added $InstallDir to the user PATH (open a new terminal)"
        }
    }
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
