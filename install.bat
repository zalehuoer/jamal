@echo off
chcp 65001 >nul 2>&1
echo ========================================
echo   JamalC2 Setup Script
echo ========================================
echo.

:: Check if VS Build Tools directory exists (any version)
echo [1/5] Checking C++ Build Tools...

:: Check VS 2022
if exist "%ProgramFiles%\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC" (
    echo [OK] VS 2022 Build Tools found
    goto :check_rust
)
if exist "%ProgramFiles(x86)%\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC" (
    echo [OK] VS 2022 Build Tools found
    goto :check_rust
)

:: Check VS 2019 (version 16.x, folder name might be "16" or "2019")
if exist "%ProgramFiles%\Microsoft Visual Studio\2019\BuildTools\VC\Tools\MSVC" (
    echo [OK] VS 2019 Build Tools found
    goto :check_rust
)
if exist "%ProgramFiles(x86)%\Microsoft Visual Studio\2019\BuildTools\VC\Tools\MSVC" (
    echo [OK] VS 2019 Build Tools found
    goto :check_rust
)

:: Check older versions (folder might just be numbered like "18")
for %%v in (18 17 16 15) do (
    if exist "%ProgramFiles(x86)%\Microsoft Visual Studio\%%v\BuildTools\VC\Tools\MSVC" (
        echo [OK] VS Build Tools found (version %%v)
        goto :check_rust
    )
)

:: Not found - show manual install instructions
echo [X] C++ Build Tools not found!
echo.
echo ========================================
echo   MANUAL INSTALLATION REQUIRED
echo ========================================
echo.
echo 1. Download Build Tools from:
echo    https://visualstudio.microsoft.com/visual-cpp-build-tools/
echo.
echo 2. Run the installer
echo.
echo 3. Select "Desktop development with C++"
echo.
echo 4. Click "Install" and wait (10-20 min)
echo.
echo 5. RESTART your computer
echo.
echo 6. Run this script again
echo ========================================
echo.
echo Press any key to open the download page...
pause >nul
start https://visualstudio.microsoft.com/visual-cpp-build-tools/
exit /b 0

:check_rust
:: Check Rust
echo [2/5] Checking Rust...
rustc --version >nul 2>&1
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
node --version >nul 2>&1
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
