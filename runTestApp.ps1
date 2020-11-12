Push-Location $PSScriptRoot
try {
    cargo build
    if (Test-Path .\target\testapp) {
        Remove-Item -Recurse .\target\testapp
    }
    mkdir .\target\testapp > $null

    Copy-Item -Recurse .\testapp\assets\* .\target\testapp
    Copy-Item .\target\debug\testapp.exe .\target\testapp

    cargo run -p sexe -- .\target\debug\loader.exe .\target\testapp .\target\testapp_packaged.exe
    .\target\testapp_packaged.exe
} finally {
    Pop-Location
}
