$local:ErrorActionPreference = 'Stop'

Push-Location $PSScriptRoot
try {
    if (Test-Path .\target\testapp) {
        Remove-Item -Recurse .\target\testapp
    }
    if (Test-Path .\target\extracted) {
        Remove-Item -Recurse .\target\extracted
    }

    cargo build
    if (-not $?) { Write-Error 'Command failed' }
    cargo clippy --all-targets --all-features -- -D warnings
    if (-not $?) { Write-Error 'Command failed' }
    mkdir .\target\testapp > $null

    Copy-Item -Recurse .\testapp\assets\* .\target\testapp
    Copy-Item .\target\debug\testapp.exe .\target\testapp

    .\target\debug\sexe.exe pack .\target\debug\sexe_loader.exe .\target\testapp .\target\testapp_packaged.exe
    if (-not $?) { Write-Error 'Command failed' }
    .\target\testapp_packaged.exe arg1 arg2 arg3
    if (-not $?) { Write-Error 'Command failed' }

    .\target\debug\sexe.exe swap .\target\testapp_packaged.exe .\target\debug\sexe_loader.exe .\target\testapp_packaged.exe
    if (-not $?) { Write-Error 'Command failed' }
    .\target\testapp_packaged.exe arg1 arg2 arg3
    if (-not $?) { Write-Error 'Command failed' }
    .\target\debug\sexe.exe swap .\target\testapp_packaged.exe .\target\debug\sexe_loader.exe
    if (-not $?) { Write-Error 'Command failed' }
    .\target\testapp_packaged.exe arg1 arg2 arg3
    if (-not $?) { Write-Error 'Command failed' }

    .\target\debug\sexe.exe list .\target\testapp_packaged.exe
    if (-not $?) { Write-Error 'Command failed' }

    .\target\debug\sexe.exe extract .\target\testapp_packaged.exe .\target\extracted
    if (-not $?) { Write-Error 'Command failed' }
    Get-ChildItem -Recurse .\target\extracted

    .\target\debug\sexe.exe check .\target\testapp_packaged.exe
    if (-not $?) { Write-Error 'Command failed' }
    .\target\debug\sexe.exe check .\target\debug\sexe.exe
    if ($?) { Write-Error 'Command should have failed' }
} finally {
    Pop-Location
}
