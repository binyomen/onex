[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [String] $Target
)

Push-Location $PSScriptRoot
try {
    cargo build
    Write-Host "Swapping $Target..."
    cargo run -p sexe -- swap $Target .\target\debug\sexe_loader.exe
    Write-Host "Done."
} finally {
    Pop-Location
}
