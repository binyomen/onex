Push-Location $PSScriptRoot
try {
    cargo build -p loader
    cargo run -p sexe -- .\target\debug\loader.exe 'hello world!' .\target\testapp.exe
} finally {
    Pop-Location
}
