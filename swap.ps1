[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [String] $Target
)

Push-Location $PSScriptRoot
try {
    cargo build
    cargo clippy --all-targets --all-features -- -D warnings
    Write-Host "Swapping $Target..."
    .\target\debug\onex.exe swap $Target .\target\debug\onex_loader.exe
    Write-Host "Done."
} finally {
    Pop-Location
}
