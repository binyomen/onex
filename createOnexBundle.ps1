$local:ErrorActionPreference = 'Stop'

Push-Location $PSScriptRoot
try {
    if (Test-Path .\target\onex_bundle) {
        Remove-Item -Recurse .\target\onex_bundle
    }
    mkdir .\target\onex_bundle > $null
    if (Test-Path .\target\onex_bundle_output) {
        Remove-Item -Recurse .\target\onex_bundle_output
    }
    mkdir .\target\onex_bundle_output > $null

    cargo build --release
    if (-not $?) { Write-Error 'Build failed' }

    Copy-Item .\target\release\onex.exe .\target\onex_bundle
    Copy-Item .\target\release\onex_loader.exe .\target\onex_bundle
    Write-Output 'onex.exe' > .\target\onex_bundle\onex_run

    .\target\release\onex.exe pack .\target\onex_bundle .\target\onex_bundle_output\onex.exe --loader .\target\release\onex_loader.exe
    if (-not $?) { Write-Error 'Packaging failed' }
} finally {
    Pop-Location
}
