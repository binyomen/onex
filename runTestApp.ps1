Push-Location $PSScriptRoot
try {
    cargo build -p loader
    cargo run -p sexe -- .\target\debug\loader.exe .\testapp .\target\testapp.exe
    .\target\testapp.exe
} finally {
    Pop-Location
}
