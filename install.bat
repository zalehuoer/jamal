@echo off
chcp 65001 >nul 2>&1
echo ========================================
echo   JamalC2 Setup Script
echo ========================================
echo.

:: Check Visual Studio Build Tools (check installation folder)
echo [1/5] Checking Visual Studio Build Tools...
if exist "%ProgramFiles%\Microsoft Visual Studio\2022\BuildTools" (
    echo [OK] VS Build Tools 2022 installed
) else if exist "%ProgramFiles(x86)%\Microsoft Visual Studio\2022\BuildTools" (
    echo [OK] VS Build Tools 2022 installed
) else if exist "%ProgramFiles%\Microsoft Visual Studio\2019\BuildTools" (
    echo [OK] VS Build Tools 2019 installed
) else (
    echo [!] VS Build Tools not found, installing...
    echo [i] This may take 10-20 minutes...
    winget install Microsoft.VisualStudio.2022.BuildTools --override "--add Microsoft.VisualStudio.Workload.VCTools --quiet --wait" --accept-source-agreements --accept-package-agreements
    echo [i] Please restart your computer and run this script again
    pause
    exit /b 0
)

:: Check Rust
echo [2/5] Checking Rust...
where rustc >nul 2>&1
if %errorlevel% neq 0 (
    echo [!] Rust not installed, installing...
    winget install Rustlang.Rustup -e --accept-source-agreements --accept-package-agreements
    echo [i] Please restart terminal and run this script again
    pause
    exit /b 0
) else (
    echo [OK] Rust installed
)

:: Check Node.js
echo [3/5] Checking Node.js...
where node >nul 2>&1
if %errorlevel% neq 0 (
    echo [!] Node.js not installed, installing...
    winget install OpenJS.NodeJS.LTS -e --accept-source-agreements --accept-package-agreements
    echo [i] Please restart terminal and run this script again
    pause
    exit /b 0
) else (
    echo [OK] Node.js installed
)

:: Install frontend dependencies
echo [4/5] Installing Server dependencies...
cd /d "%~dp0server"
if not exist node_modules (
    call npm install
    if %errorlevel% neq 0 (
        echo [X] npm install failed
        pause
        exit /b 1
    )
) else (
    echo [OK] Dependencies exist, skipping
)
cd /d "%~dp0"

:: Done
echo [5/5] Installation complete!
echo.
echo ========================================
echo   Usage:
echo   1. cd server
echo   2. npm run tauri dev
echo ========================================
echo.
pause
