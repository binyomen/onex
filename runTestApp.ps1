Push-Location $PSScriptRoot
try {
    if (Test-Path .\target\testapp) {
        Remove-Item -Recurse .\target\testapp
    }
    if (Test-Path .\target\extracted) {
        Remove-Item -Recurse .\target\extracted
    }

    cargo build
    cargo clippy --all-targets --all-features -- -D warnings
    mkdir .\target\testapp > $null

    Copy-Item -Recurse .\testapp\assets\* .\target\testapp
    Copy-Item .\target\debug\testapp.exe .\target\testapp

    cargo run -p sexe -- pack .\target\debug\sexe_loader.exe .\target\testapp .\target\testapp_packaged.exe
    .\target\testapp_packaged.exe arg1 arg2 arg3

    cargo run -p sexe -- swap .\target\testapp_packaged.exe .\target\debug\sexe_loader.exe .\target\testapp_packaged.exe
    .\target\testapp_packaged.exe arg1 arg2 arg3
    cargo run -p sexe -- swap .\target\testapp_packaged.exe .\target\debug\sexe_loader.exe
    .\target\testapp_packaged.exe arg1 arg2 arg3

    cargo run -p sexe -- list .\target\testapp_packaged.exe

    cargo run -p sexe -- extract .\target\testapp_packaged.exe .\target\extracted
    Get-ChildItem -Recurse .\target\extracted
} finally {
    Pop-Location
}
