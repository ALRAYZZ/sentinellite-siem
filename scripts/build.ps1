$ErrorActionPreference = 'Stop'

function Invoke-CheckedCommand {
    param(
        [Parameter(Mandatory = $true)][string]$Executable,
        [Parameter(Mandatory = $true)][string[]]$Arguments
    )

    & $Executable @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Executable $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
    }
}

Push-Location (Join-Path $PSScriptRoot '..')
try {
    Push-Location 'frontend'
    try {
        Invoke-CheckedCommand -Executable 'npm.cmd' -Arguments @('install')
        Invoke-CheckedCommand -Executable 'npm.cmd' -Arguments @('run', 'build')
    } finally {
        Pop-Location
    }

    # Format first so a fresh checkout does not fail merely because rustfmt has not run.
    Invoke-CheckedCommand -Executable 'cargo' -Arguments @('fmt')
    Invoke-CheckedCommand -Executable 'cargo' -Arguments @('test')
    Invoke-CheckedCommand -Executable 'cargo' -Arguments @('clippy', '--all-targets', '--all-features', '--', '-D', 'warnings')
    Invoke-CheckedCommand -Executable 'cargo' -Arguments @('build', '--release')
} finally {
    Pop-Location
}
