$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    if (-not (Test-Path 'frontend/dist/index.html')) {
        throw 'Frontend build not found. Run scripts/build.ps1 first.'
    }
    cargo run --release
} finally {
    Pop-Location
}
