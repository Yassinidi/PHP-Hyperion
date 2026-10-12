# =============================================================================
# ⚡ PHP-Hyperion Native Windows Installer (PowerShell)
# =============================================================================

Write-Host "⚡ PHP-Hyperion Native Windows Installer" -ForegroundColor Cyan
Write-Host "Detecting environment..." -ForegroundColor Yellow

$InstallDir = "$env:LOCALAPPDATA\PHP-Hyperion"
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$TargetExe = "$InstallDir\hyperion.exe"

# If local precompiled binary exists in deploy\ or bin\
if (Test-Path "$ScriptDir\deploy\hyperion.exe") {
    Write-Host "[INFO] Installing native Windows binary from deploy\hyperion.exe..." -ForegroundColor Cyan
    Copy-Item "$ScriptDir\deploy\hyperion.exe" -Destination $TargetExe -Force
} elseif (Test-Path "$ScriptDir\bin\hyperion.exe") {
    Write-Host "[INFO] Installing native Windows binary from bin\hyperion.exe..." -ForegroundColor Cyan
    Copy-Item "$ScriptDir\bin\hyperion.exe" -Destination $TargetExe -Force
} else {
    Write-Host "[INFO] Downloading native Windows binary from GitHub..." -ForegroundColor Cyan
    $Url = "https://raw.githubusercontent.com/Yassinidi/PHP-Hyperion/main/deploy/hyperion.exe"
    Invoke-WebRequest -Uri $Url -OutFile $TargetExe
}

# Add to user PATH if not present
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    Write-Host "[INFO] Added $InstallDir to user PATH." -ForegroundColor Green
    $env:Path = "$env:Path;$InstallDir"
}

Write-Host "`n✓ Installation Successful!" -ForegroundColor Green
Write-Host "Installed executable: $TargetExe`n" -ForegroundColor Cyan

& "$TargetExe" -v

Write-Host "`nQuick Start:" -ForegroundColor Cyan
Write-Host "  hyperion script.php"
Write-Host "  hyperion -S 127.0.0.1:8000"
