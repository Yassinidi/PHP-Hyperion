# =============================================================================
# ⚡ PHP-Hyperion Windows Installer (PowerShell)
# =============================================================================

Write-Host "⚡ PHP-Hyperion Windows Installer" -ForegroundColor Cyan
Write-Host "Detecting environment..." -ForegroundColor Yellow

$InstallDir = "$env:ProgramFiles\PHP-Hyperion"
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$Repo = "Yassinidi/PHP-Hyperion"
$TargetExe = "$InstallDir\hyperion.exe"

# If local binary exists in deploy\
if (Test-Path "deploy\hyperion.exe") {
    Copy-Item "deploy\hyperion.exe" -Destination $TargetExe -Force
} else {
    Write-Host "Downloading latest Hyperion release from GitHub..." -ForegroundColor Cyan
    $Url = "https://github.com/$Repo/releases/latest/download/hyperion-windows-x86_64.exe"
    try {
        Invoke-WebRequest -Uri $Url -OutFile $TargetExe
    } catch {
        Write-Host "For native Windows, Docker or WSL2 is recommended for maximum POSIX event loop performance." -ForegroundColor Yellow
        Write-Host "Run: wsl curl -fsSL https://raw.githubusercontent.com/$Repo/main/install.sh | bash" -ForegroundColor Green
    }
}

# Add to user PATH if not present
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    Write-Host "Added $InstallDir to user PATH." -ForegroundColor Green
}

Write-Host "✓ Installation finished. Run 'hyperion -v' to verify." -ForegroundColor Green
