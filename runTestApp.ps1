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

    cargo run -p sexe -- pack .\target\debug\sexe_loader.exe .\target\testapp .\target\testapp_packaged.exe
    if (-not $?) { Write-Error 'Command failed' }
    .\target\testapp_packaged.exe arg1 arg2 arg3
    if (-not $?) { Write-Error 'Command failed' }

    cargo run -p sexe -- swap .\target\testapp_packaged.exe .\target\debug\sexe_loader.exe .\target\testapp_packaged.exe
    if (-not $?) { Write-Error 'Command failed' }
    .\target\testapp_packaged.exe arg1 arg2 arg3
    if (-not $?) { Write-Error 'Command failed' }
    cargo run -p sexe -- swap .\target\testapp_packaged.exe .\target\debug\sexe_loader.exe
    if (-not $?) { Write-Error 'Command failed' }
    .\target\testapp_packaged.exe arg1 arg2 arg3
    if (-not $?) { Write-Error 'Command failed' }

    cargo run -p sexe -- list .\target\testapp_packaged.exe
    if (-not $?) { Write-Error 'Command failed' }

    cargo run -p sexe -- extract .\target\testapp_packaged.exe .\target\extracted
    if (-not $?) { Write-Error 'Command failed' }
    Get-ChildItem -Recurse .\target\extracted
} finally {
    Pop-Location
}
