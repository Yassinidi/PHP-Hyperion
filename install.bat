@echo off
rem =============================================================================
rem ⚡ PHP-Hyperion Windows Batch Installer (Command Prompt / Double Click)
rem =============================================================================

echo.
echo ========================================================
echo   PHP-Hyperion Native Windows Installer
echo ========================================================
echo.

set "TARGET_DIR=%LOCALAPPDATA%\PHP-Hyperion"

if not exist "%TARGET_DIR%" (
    mkdir "%TARGET_DIR%"
)

if exist "%~dp0deploy\hyperion.exe" (
    echo [INFO] Installing pre-compiled Windows binary from deploy\hyperion.exe...
    copy /Y "%~dp0deploy\hyperion.exe" "%TARGET_DIR%\hyperion.exe" >nul
) else if exist "%~dp0bin\hyperion.exe" (
    echo [INFO] Installing pre-compiled Windows binary from bin\hyperion.exe...
    copy /Y "%~dp0bin\hyperion.exe" "%TARGET_DIR%\hyperion.exe" >nul
) else (
    echo [INFO] Downloading latest Hyperion release from GitHub...
    curl -fsSL "https://raw.githubusercontent.com/Yassinidi/PHP-Hyperion/main/deploy/hyperion.exe" -o "%TARGET_DIR%\hyperion.exe"
)

if not exist "%TARGET_DIR%\hyperion.exe" (
    echo [ERROR] Failed to install hyperion.exe. Please check permissions or network connection.
    pause
    exit /b 1
)

rem Add to user PATH if not already in PATH
echo %PATH% | findstr /I /C:"%TARGET_DIR%" >nul
if errorlevel 1 (
    echo [INFO] Adding %TARGET_DIR% to user PATH...
    setx PATH "%PATH%;%TARGET_DIR%" >nul
    set "PATH=%PATH%;%TARGET_DIR%"
)

echo.
echo ========================================================
echo   Installation Successful!
echo ========================================================
echo Installed executable: %TARGET_DIR%\hyperion.exe
echo.
"%TARGET_DIR%\hyperion.exe" -v
echo.
echo You can now run 'hyperion -S 127.0.0.1:8000' from any Command Prompt or PowerShell!
echo.
pause
