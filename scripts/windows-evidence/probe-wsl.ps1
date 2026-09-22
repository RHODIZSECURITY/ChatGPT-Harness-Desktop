<#
.SYNOPSIS
    Captures the exact bytes `wsl.exe` writes, for Windows certification evidence.

.DESCRIPTION
    The broker decodes `wsl.exe` output as UTF-16LE and scans it for a
    major.minor.patch triple. Both behaviours are so far certified only against
    bytes this repository synthesized itself: no real `wsl.exe` output has ever
    been observed. This script closes that gap.

    It captures the RAW BYTE STREAM, never decoded text. PowerShell re-encodes
    anything that passes through a string, which would destroy the one property
    under test, so stdout and stderr are read straight off the process base
    stream and stored as base64.

    This script asks the machine nothing: it adds no hostname, no user name and
    no paths of its own, and the only text it invents is the -Label you pass.

    But it records what wsl.exe prints, and wsl.exe is chattier than its name
    suggests. A real capture disclosed all of this:

      - the exact Windows build and UBR ("Versión de Windows: 10.0.26200.9457")
      - the WSL kernel and component versions (WSLg, MSRDC, Direct3D, DXCore),
        one of which carries a servicing branch string
      - the default distro name, from --status ("Ubuntu")
      - the display language, because wsl.exe localizes its own labels

    None of that is a secret, and all of it is the point: it is what makes the
    capture evidence about a specific Windows rather than a shape. It is still
    more than "nothing", so READ THE FILE BEFORE COMMITTING IT and decide for
    yourself. Anything you are not willing to publish, do not commit.

.PARAMETER Label
    Free-text note recorded with the capture, e.g. "win11-24h2-clean-vm".
    The operator chooses what, if anything, this discloses.

.PARAMETER OutFile
    Where to write the evidence JSON. Defaults to
    evidence/windows/wsl-probe-<label>.json relative to the repository root.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\windows-evidence\probe-wsl.ps1 -Label win11-24h2
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[a-z0-9][a-z0-9.-]{0,62}$')]
    [string]$Label,

    [string]$OutFile
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# The probes are fixed on purpose. An operator-supplied command line would make
# the evidence unreproducible, and these are the two invocations the broker
# actually performs.
$Probes = @(
    @{ id = 'wsl-version'; arguments = @('--version') },
    @{ id = 'wsl-status';  arguments = @('--status')  }
)

function Invoke-ByteProbe {
    param(
        [string]$Executable,
        [string[]]$Arguments
    )

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $Executable
    # ArgumentList does not exist on the .NET Framework that Windows PowerShell
    # 5.1 runs on, so the fixed flags above are joined into the legacy string.
    # They are literal, space-free switches, so no quoting is required.
    $psi.Arguments = ($Arguments -join ' ')
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true

    $proc = [System.Diagnostics.Process]::Start($psi)

    # Copy both pipes concurrently. Draining one to completion before touching
    # the other deadlocks as soon as the untouched pipe's buffer fills.
    $outBuffer = New-Object System.IO.MemoryStream
    $errBuffer = New-Object System.IO.MemoryStream
    $outCopy = $proc.StandardOutput.BaseStream.CopyToAsync($outBuffer)
    $errCopy = $proc.StandardError.BaseStream.CopyToAsync($errBuffer)

    if (-not $proc.WaitForExit(60000)) {
        $proc.Kill()
        throw "$Executable $($Arguments -join ' ') did not exit within 60s; refusing to record a partial capture"
    }
    if (-not $outCopy.Wait(10000) -or -not $errCopy.Wait(10000)) {
        throw "$Executable $($Arguments -join ' ') exited but its output could not be drained; refusing to record a partial capture"
    }

    return [ordered]@{
        executed         = $true
        exitCode         = $proc.ExitCode
        stdoutBase64     = [Convert]::ToBase64String($outBuffer.ToArray())
        stdoutByteLength = $outBuffer.Length
        stderrBase64     = [Convert]::ToBase64String($errBuffer.ToArray())
        stderrByteLength = $errBuffer.Length
    }
}

$wsl = Get-Command 'wsl.exe' -CommandType Application -ErrorAction SilentlyContinue |
    Select-Object -First 1

$results = [ordered]@{}
foreach ($probe in $Probes) {
    if ($null -eq $wsl) {
        # WSL absent is itself a certified route (task 10.2), so it is recorded
        # as evidence rather than treated as a failure of the probe.
        $results[$probe.id] = [ordered]@{
            executed = $false
            reason   = 'wsl.exe not found on PATH'
        }
        continue
    }
    $results[$probe.id] = Invoke-ByteProbe -Executable $wsl.Source -Arguments $probe.arguments
    $results[$probe.id]['arguments'] = $probe.arguments
}

$document = [ordered]@{
    schema     = 'rhodiz.harness.windows-evidence/wsl-probe/1'
    label      = $Label
    capturedAt = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    wslPresent = ($null -ne $wsl)
    probes     = $results
}

if (-not $OutFile) {
    # This script is meant to be copied onto a Windows host on its own, so the
    # repository layout cannot be assumed. Write into evidence/windows/ only
    # when the script really is sitting inside a checkout; otherwise write
    # beside the script, where the operator will actually find the file.
    $here = if ($PSScriptRoot) { $PSScriptRoot } else { (Get-Location).Path }
    $repoRoot = Split-Path -Parent (Split-Path -Parent $here)
    # src-tauri/broker-core is what consumes these captures, so its presence is
    # the marker that this really is a checkout and not a loose copy.
    $inCheckout = $repoRoot -and
        (Test-Path -LiteralPath (Join-Path $repoRoot 'src-tauri/broker-core'))
    $targetDir = if ($inCheckout) { Join-Path $repoRoot 'evidence/windows' } else { $here }
    $OutFile = Join-Path $targetDir "wsl-probe-$Label.json"
}
$outDir = Split-Path -Parent $OutFile
if ($outDir -and -not (Test-Path -LiteralPath $outDir)) {
    [void](New-Item -ItemType Directory -Path $outDir -Force)
}

$json = $document | ConvertTo-Json -Depth 6
[System.IO.File]::WriteAllText($OutFile, $json, (New-Object System.Text.UTF8Encoding($false)))

Write-Host "wsl.exe present: $($document.wslPresent)"
foreach ($key in $results.Keys) {
    $entry = $results[$key]
    if ($entry.executed) {
        Write-Host ("{0}: exit={1} stdout={2}B stderr={3}B" -f $key, $entry.exitCode, $entry.stdoutByteLength, $entry.stderrByteLength)
    } else {
        Write-Host ("{0}: not executed ({1})" -f $key, $entry.reason)
    }
}
Write-Host "Evidence written to $OutFile"
Write-Host "Review it before committing: wsl.exe --status prints your default distro name."
