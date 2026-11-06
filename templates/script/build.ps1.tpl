# Builds the plugin and stages a package directory in pkg/.
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
New-Item -ItemType Directory -Force pkg | Out-Null
Copy-Item 'manifest.json' 'pkg\manifest.json' -Force
Copy-Item '{{script}}' 'pkg\{{script}}' -Force
Write-Host 'plugin staged in pkg/'
